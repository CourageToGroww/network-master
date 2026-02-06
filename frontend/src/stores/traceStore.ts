import { create } from 'zustand';
import type { HopRealtimeData, HopRunningStats, LiveHopData } from '../types';

// ─── Ring Buffer ──────────────────────────────────────────
const RING_CAPACITY = 3600; // ~1 hour at 1 sample/sec

/** Fixed-capacity ring buffer backed by Float64Array for chart data. */
class RingBuffer {
  readonly data: Float64Array;
  readonly capacity: number;
  head = 0;
  len = 0;

  constructor(capacity: number = RING_CAPACITY) {
    this.capacity = capacity;
    this.data = new Float64Array(capacity);
    this.data.fill(NaN);
  }

  push(value: number): void {
    this.data[this.head] = value;
    this.head = (this.head + 1) % this.capacity;
    if (this.len < this.capacity) this.len++;
  }

  /** Returns data in chronological order as a new Float64Array. */
  toSorted(): Float64Array {
    if (this.len === 0) return new Float64Array(0);
    const out = new Float64Array(this.len);
    const start = (this.head - this.len + this.capacity) % this.capacity;
    for (let i = 0; i < this.len; i++) {
      out[i] = this.data[(start + i) % this.capacity];
    }
    return out;
  }
}

// ─── Time Series ──────────────────────────────────────────
interface TraceTimeSeries {
  timestamps: RingBuffer;
  /** Per-hop RTT in ms (NaN = lost packet). Key = hop_number. */
  hopRtt: Map<number, RingBuffer>;
  /** Per-hop running loss% (0-100). Key = hop_number. */
  hopLoss: Map<number, RingBuffer>;
  /** Per-hop jitter in ms. Key = hop_number. */
  hopJitter: Map<number, RingBuffer>;
  roundCount: number;
}

function createTimeSeries(): TraceTimeSeries {
  return {
    timestamps: new RingBuffer(),
    hopRtt: new Map(),
    hopLoss: new Map(),
    hopJitter: new Map(),
    roundCount: 0,
  };
}

// ─── Types ────────────────────────────────────────────────
interface HopUpdate {
  hopNumber: number;
  ip: string | null;
  hostname: string | null;
  rttUs: number | null;
  isLost: boolean;
  stats: HopRunningStats;
}

interface ActiveTrace {
  agentId: string;
  targetId: string;
  hops: Map<number, HopRealtimeData>;
  timeSeries: TraceTimeSeries;
}

export interface TimeSeriesData {
  hopNumbers: number[];
  /** [timestamps, hop1_rtt, hop2_rtt, ...] - all Float64Array in chronological order */
  data: Float64Array[];
}

/** Compute ITU-T E-model MOS score from latency, jitter, and loss */
export function computeMOS(avgRttMs: number, jitterMs: number, lossPct: number): number {
  // Simplified E-model: effective latency includes jitter buffer
  const effectiveLatency = avgRttMs + jitterMs * 2 + 10;

  // Calculate R factor
  let r = 93.2 - effectiveLatency / 40;

  // Loss impact
  r -= lossPct * 2.5;

  // Clamp R to [0, 100]
  r = Math.max(0, Math.min(100, r));

  // Convert R to MOS (1-5 scale)
  if (r < 0) return 1.0;
  const mos = 1 + 0.035 * r + r * (r - 60) * 7e-6;
  return Math.max(1.0, Math.min(5.0, mos));
}

interface TraceState {
  traces: Map<string, ActiveTrace>;

  initTrace: (agentId: string, targetId: string) => void;
  pushRound: (agentId: string, targetId: string, sentAt: string, hops: LiveHopData[]) => void;
  updateHop: (agentId: string, targetId: string, hop: HopUpdate) => void;
  clearTrace: (agentId: string, targetId: string) => void;
  getHopsArray: (agentId: string, targetId: string) => HopRealtimeData[];
  getTimeSeries: (agentId: string, targetId: string) => TimeSeriesData | null;
  getLossTimeSeries: (agentId: string, targetId: string) => TimeSeriesData | null;
  getJitterTimeSeries: (agentId: string, targetId: string) => TimeSeriesData | null;
  getRoundCount: (agentId: string, targetId: string) => number;
}

function traceKey(agentId: string, targetId: string): string {
  return `${agentId}:${targetId}`;
}

function buildHopData(hop: HopUpdate): HopRealtimeData {
  const rttMs = hop.rttUs != null ? hop.rttUs / 1000 : 0;
  const avgMs = hop.stats.avg_rtt_us / 1000;
  const jitterMs = hop.stats.jitter_avg_us / 1000;
  const lossPct = hop.stats.loss_pct;
  return {
    hopNumber: hop.hopNumber,
    ip: hop.ip,
    hostname: hop.hostname,
    lossPct,
    sent: hop.stats.sample_count,
    recv: hop.stats.sample_count - Math.round(lossPct * hop.stats.sample_count / 100),
    bestMs: hop.stats.min_rtt_us / 1000,
    avgMs,
    worstMs: hop.stats.max_rtt_us / 1000,
    lastMs: rttMs,
    jitterMs,
    mos: computeMOS(avgMs, jitterMs, lossPct),
  };
}

export const useTraceStore = create<TraceState>()((set, get) => ({
  traces: new Map(),

  initTrace: (agentId, targetId) =>
    set((state) => {
      const key = traceKey(agentId, targetId);
      if (state.traces.has(key)) return state; // Already initialized
      const next = new Map(state.traces);
      next.set(key, {
        agentId,
        targetId,
        hops: new Map(),
        timeSeries: createTimeSeries(),
      });
      return { traces: next };
    }),

  pushRound: (agentId, targetId, sentAt, hops) =>
    set((state) => {
      const key = traceKey(agentId, targetId);
      let trace = state.traces.get(key);
      if (!trace) {
        trace = { agentId, targetId, hops: new Map(), timeSeries: createTimeSeries() };
      }

      const ts = trace.timeSeries;
      const timestamp = new Date(sentAt).getTime() / 1000; // seconds since epoch
      ts.timestamps.push(timestamp);

      const newHops = new Map(trace.hops);

      for (const hop of hops) {
        // Update hop stats
        newHops.set(hop.hop_number, buildHopData({
          hopNumber: hop.hop_number,
          ip: hop.ip_address,
          hostname: hop.hostname,
          rttUs: hop.rtt_us,
          isLost: hop.is_lost,
          stats: hop.stats,
        }));

        // Push RTT to time series ring buffer
        let rttRing = ts.hopRtt.get(hop.hop_number);
        if (!rttRing) {
          rttRing = new RingBuffer();
          ts.hopRtt.set(hop.hop_number, rttRing);
        }
        rttRing.push(hop.is_lost || hop.rtt_us == null ? NaN : hop.rtt_us / 1000);

        // Push loss% to ring buffer
        let lossRing = ts.hopLoss.get(hop.hop_number);
        if (!lossRing) {
          lossRing = new RingBuffer();
          ts.hopLoss.set(hop.hop_number, lossRing);
        }
        lossRing.push(hop.stats.loss_pct);

        // Push jitter to ring buffer
        let jitterRing = ts.hopJitter.get(hop.hop_number);
        if (!jitterRing) {
          jitterRing = new RingBuffer();
          ts.hopJitter.set(hop.hop_number, jitterRing);
        }
        jitterRing.push(hop.stats.jitter_avg_us / 1000);
      }

      ts.roundCount++;

      const next = new Map(state.traces);
      next.set(key, { ...trace, hops: newHops });
      return { traces: next };
    }),

  updateHop: (agentId, targetId, hop) =>
    set((state) => {
      const key = traceKey(agentId, targetId);
      let trace = state.traces.get(key);
      if (!trace) {
        trace = { agentId, targetId, hops: new Map(), timeSeries: createTimeSeries() };
      }

      const newHops = new Map(trace.hops);
      newHops.set(hop.hopNumber, buildHopData(hop));

      const next = new Map(state.traces);
      next.set(key, { ...trace, hops: newHops });
      return { traces: next };
    }),

  clearTrace: (agentId, targetId) =>
    set((state) => {
      const next = new Map(state.traces);
      next.delete(traceKey(agentId, targetId));
      return { traces: next };
    }),

  getHopsArray: (agentId, targetId) => {
    const trace = get().traces.get(traceKey(agentId, targetId));
    if (!trace) return [];
    return Array.from(trace.hops.values()).sort((a, b) => a.hopNumber - b.hopNumber);
  },

  getTimeSeries: (agentId, targetId) => {
    const trace = get().traces.get(traceKey(agentId, targetId));
    if (!trace || trace.timeSeries.roundCount === 0) return null;

    const ts = trace.timeSeries;
    const timestamps = ts.timestamps.toSorted();
    const hopNumbers = Array.from(ts.hopRtt.keys()).sort((a, b) => a - b);
    const data: Float64Array[] = [timestamps];
    for (const hopNum of hopNumbers) {
      data.push(ts.hopRtt.get(hopNum)!.toSorted());
    }
    return { hopNumbers, data };
  },

  getLossTimeSeries: (agentId, targetId) => {
    const trace = get().traces.get(traceKey(agentId, targetId));
    if (!trace || trace.timeSeries.roundCount === 0) return null;

    const ts = trace.timeSeries;
    const timestamps = ts.timestamps.toSorted();
    const hopNumbers = Array.from(ts.hopLoss.keys()).sort((a, b) => a - b);
    const data: Float64Array[] = [timestamps];
    for (const hopNum of hopNumbers) {
      data.push(ts.hopLoss.get(hopNum)!.toSorted());
    }
    return { hopNumbers, data };
  },

  getJitterTimeSeries: (agentId, targetId) => {
    const trace = get().traces.get(traceKey(agentId, targetId));
    if (!trace || trace.timeSeries.roundCount === 0) return null;

    const ts = trace.timeSeries;
    const timestamps = ts.timestamps.toSorted();
    const hopNumbers = Array.from(ts.hopJitter.keys()).sort((a, b) => a - b);
    const data: Float64Array[] = [timestamps];
    for (const hopNum of hopNumbers) {
      data.push(ts.hopJitter.get(hopNum)!.toSorted());
    }
    return { hopNumbers, data };
  },

  getRoundCount: (agentId, targetId) => {
    const trace = get().traces.get(traceKey(agentId, targetId));
    return trace?.timeSeries.roundCount ?? 0;
  },
}));
