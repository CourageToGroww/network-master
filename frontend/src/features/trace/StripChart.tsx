import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { useTraceStore } from '../../stores/traceStore';

interface StripChartProps {
  agentId: string;
  targetId: string;
  selectedHop: number | null;
}

// PingPlotter-style color gradient: green → yellow → orange → red → dark red
function rttToColor(ms: number): string {
  if (Number.isNaN(ms) || ms <= 0) return '#111111'; // lost / invalid = near black
  if (ms < 1) return '#059669';     // emerald-600 - excellent
  if (ms < 5) return '#16a34a';     // green-600
  if (ms < 10) return '#22c55e';    // green-500
  if (ms < 20) return '#4ade80';    // green-400
  if (ms < 40) return '#a3e635';    // lime-400
  if (ms < 60) return '#facc15';    // yellow-400
  if (ms < 80) return '#fbbf24';    // amber-400
  if (ms < 100) return '#f59e0b';   // amber-500
  if (ms < 150) return '#f97316';   // orange-500
  if (ms < 200) return '#ef4444';   // red-500
  if (ms < 300) return '#dc2626';   // red-600
  if (ms < 500) return '#b91c1c';   // red-700
  return '#7f1d1d';                  // red-900 - critical
}

const STRIP_WIDTH = 3;
const ROW_HEIGHT = 18;
const LABEL_WIDTH = 55;

export function StripChart({ agentId, targetId, selectedHop }: StripChartProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const [tooltip, setTooltip] = useState<{
    x: number; y: number; hop: number; round: number; rtt: number;
  } | null>(null);

  const roundCount = useTraceStore(
    useCallback(
      (s) => s.getRoundCount(agentId, targetId),
      [agentId, targetId],
    ),
  );

  const timeSeries = useMemo(() => {
    return useTraceStore.getState().getTimeSeries(agentId, targetId);
  }, [agentId, targetId, roundCount]);

  // Draw the strip chart
  useEffect(() => {
    const canvas = canvasRef.current;
    const container = containerRef.current;
    if (!canvas || !container || !timeSeries || timeSeries.data.length < 2) return;

    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    const { hopNumbers, data } = timeSeries;
    const numRounds = data[0].length;
    const numHops = hopNumbers.length;

    const containerWidth = container.clientWidth;
    const containerHeight = container.clientHeight;
    const dpr = window.devicePixelRatio || 1;

    canvas.width = containerWidth * dpr;
    canvas.height = containerHeight * dpr;
    canvas.style.width = `${containerWidth}px`;
    canvas.style.height = `${containerHeight}px`;
    ctx.scale(dpr, dpr);

    // Background
    ctx.fillStyle = '#0f172a';
    ctx.fillRect(0, 0, containerWidth, containerHeight);

    const visibleRounds = Math.floor((containerWidth - LABEL_WIDTH) / STRIP_WIDTH);
    const startRound = Math.max(0, numRounds - visibleRounds);

    // Draw hop rows
    for (let hopIdx = 0; hopIdx < numHops; hopIdx++) {
      const hopNum = hopNumbers[hopIdx];
      const y = hopIdx * ROW_HEIGHT;

      // Selected hop highlight
      if (selectedHop === hopNum) {
        ctx.fillStyle = 'rgba(59, 130, 246, 0.12)';
        ctx.fillRect(0, y, containerWidth, ROW_HEIGHT);
      }

      // Alternating row bg
      if (hopIdx % 2 === 0 && selectedHop !== hopNum) {
        ctx.fillStyle = 'rgba(148, 163, 184, 0.03)';
        ctx.fillRect(0, y, containerWidth, ROW_HEIGHT);
      }

      // Draw hop label
      ctx.font = '10px "JetBrains Mono", "Cascadia Code", monospace';
      ctx.textBaseline = 'middle';
      ctx.fillStyle = selectedHop === hopNum ? '#3b82f6' : '#64748b';
      ctx.fillText(`${hopNum}`, 6, y + ROW_HEIGHT / 2);
    }

    // Draw strip columns
    for (let roundIdx = startRound; roundIdx < numRounds; roundIdx++) {
      const x = LABEL_WIDTH + (roundIdx - startRound) * STRIP_WIDTH;

      for (let hopIdx = 0; hopIdx < numHops; hopIdx++) {
        const rttMs = data[hopIdx + 1][roundIdx];
        const y = hopIdx * ROW_HEIGHT;

        ctx.fillStyle = rttToColor(rttMs);
        ctx.fillRect(x, y + 1, STRIP_WIDTH - 1, ROW_HEIGHT - 2);
      }
    }

    // Subtle grid lines
    ctx.strokeStyle = 'rgba(148, 163, 184, 0.06)';
    ctx.lineWidth = 1;
    for (let hopIdx = 1; hopIdx < numHops; hopIdx++) {
      const y = hopIdx * ROW_HEIGHT;
      ctx.beginPath();
      ctx.moveTo(LABEL_WIDTH, y);
      ctx.lineTo(containerWidth, y);
      ctx.stroke();
    }

    // Right edge gradient (fade to background)
    const grad = ctx.createLinearGradient(containerWidth - 20, 0, containerWidth, 0);
    grad.addColorStop(0, 'rgba(15, 23, 42, 0)');
    grad.addColorStop(1, 'rgba(15, 23, 42, 0.8)');
    ctx.fillStyle = grad;
    ctx.fillRect(containerWidth - 20, 0, 20, containerHeight);
  }, [timeSeries, roundCount, selectedHop]);

  // Mouse hover for tooltip
  const handleMouseMove = useCallback(
    (e: React.MouseEvent<HTMLCanvasElement>) => {
      if (!timeSeries || timeSeries.data.length < 2) return;
      const canvas = canvasRef.current;
      if (!canvas) return;

      const rect = canvas.getBoundingClientRect();
      const x = e.clientX - rect.left;
      const y = e.clientY - rect.top;

      if (x < LABEL_WIDTH) { setTooltip(null); return; }

      const { hopNumbers, data } = timeSeries;
      const numRounds = data[0].length;
      const visibleRounds = Math.floor((rect.width - LABEL_WIDTH) / STRIP_WIDTH);
      const startRound = Math.max(0, numRounds - visibleRounds);

      const hopIdx = Math.floor(y / ROW_HEIGHT);
      const roundIdx = startRound + Math.floor((x - LABEL_WIDTH) / STRIP_WIDTH);

      if (hopIdx >= 0 && hopIdx < hopNumbers.length && roundIdx >= 0 && roundIdx < numRounds) {
        const rtt = data[hopIdx + 1][roundIdx];
        setTooltip({
          x: e.clientX - rect.left,
          y: e.clientY - rect.top,
          hop: hopNumbers[hopIdx],
          round: roundIdx + 1,
          rtt,
        });
      } else {
        setTooltip(null);
      }
    },
    [timeSeries],
  );

  const handleMouseLeave = useCallback(() => setTooltip(null), []);

  if (!timeSeries) {
    return (
      <div className="flex items-center justify-center h-full text-[var(--text-secondary)] text-sm">
        Waiting for trace data...
      </div>
    );
  }

  return (
    <div ref={containerRef} className="relative w-full h-full overflow-hidden">
      <canvas
        ref={canvasRef}
        className="block"
        onMouseMove={handleMouseMove}
        onMouseLeave={handleMouseLeave}
      />
      {tooltip && (
        <div
          className="absolute z-20 pointer-events-none bg-[#1e293b] border border-[#334155] rounded px-2 py-1 text-[10px] text-[#e2e8f0] shadow-lg"
          style={{
            left: Math.min(tooltip.x + 12, (containerRef.current?.clientWidth ?? 300) - 120),
            top: tooltip.y - 30,
          }}
        >
          <div>Hop {tooltip.hop} | Round {tooltip.round}</div>
          <div className="font-mono">
            {Number.isNaN(tooltip.rtt) ? (
              <span className="text-red-400">Lost</span>
            ) : (
              <span>{tooltip.rtt.toFixed(2)} ms</span>
            )}
          </div>
        </div>
      )}
      {/* Color legend */}
      <div className="absolute bottom-1 right-2 flex items-center gap-0.5 text-[9px] text-[#64748b]">
        <span>0ms</span>
        {['#059669', '#22c55e', '#a3e635', '#facc15', '#f59e0b', '#ef4444', '#7f1d1d'].map((c, i) => (
          <div key={i} className="w-3 h-2 rounded-sm" style={{ backgroundColor: c }} />
        ))}
        <span>500+ms</span>
        <div className="w-3 h-2 rounded-sm bg-[#111] ml-1" />
        <span>Lost</span>
      </div>
    </div>
  );
}
