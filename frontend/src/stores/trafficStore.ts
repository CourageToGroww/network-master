import { create } from 'zustand';
import type { LiveProcessTrafficUpdate, ProcessTrafficSummary } from '../types';

const HISTORY_CAPACITY = 60; // 5 minutes at 5s intervals

interface TrafficHistoryEntry {
  timestamp: number;
  bytesInPerSec: number;
  bytesOutPerSec: number;
}

interface AgentTrafficState {
  latest: LiveProcessTrafficUpdate | null;
  /** Per-process bandwidth history for sparklines. Key = process_name */
  processHistory: Map<string, TrafficHistoryEntry[]>;
}

interface TrafficState {
  agents: Map<string, AgentTrafficState>;

  pushTraffic: (update: LiveProcessTrafficUpdate) => void;
  getLatest: (agentId: string) => LiveProcessTrafficUpdate | null;
  getProcesses: (agentId: string) => ProcessTrafficSummary[];
  getProcessHistory: (agentId: string, processName: string) => TrafficHistoryEntry[];
  clearTraffic: (agentId: string) => void;
}

export type { TrafficHistoryEntry };

export const useTrafficStore = create<TrafficState>()((set, get) => ({
  agents: new Map(),

  pushTraffic: (update) =>
    set((state) => {
      const next = new Map(state.agents);
      const existing = next.get(update.agent_id) ?? {
        latest: null,
        processHistory: new Map(),
      };

      const ts = new Date(update.captured_at).getTime() / 1000;

      // Update per-process history
      const newHistory = new Map(existing.processHistory);
      for (const proc of update.processes) {
        const history = [...(newHistory.get(proc.process_name) ?? [])];
        history.push({
          timestamp: ts,
          bytesInPerSec: proc.bytes_in_per_sec,
          bytesOutPerSec: proc.bytes_out_per_sec,
        });
        // Trim to capacity
        if (history.length > HISTORY_CAPACITY) {
          history.splice(0, history.length - HISTORY_CAPACITY);
        }
        newHistory.set(proc.process_name, history);
      }

      next.set(update.agent_id, {
        latest: update,
        processHistory: newHistory,
      });

      return { agents: next };
    }),

  getLatest: (agentId) => {
    return get().agents.get(agentId)?.latest ?? null;
  },

  getProcesses: (agentId) => {
    return get().agents.get(agentId)?.latest?.processes ?? [];
  },

  getProcessHistory: (agentId, processName) => {
    return get().agents.get(agentId)?.processHistory.get(processName) ?? [];
  },

  clearTraffic: (agentId) =>
    set((state) => {
      const next = new Map(state.agents);
      next.delete(agentId);
      return { agents: next };
    }),
}));
