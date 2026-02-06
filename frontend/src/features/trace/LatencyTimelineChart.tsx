import { useCallback, useEffect, useMemo, useRef } from 'react';
import uPlot from 'uplot';
import 'uplot/dist/uPlot.min.css';
import { useTraceStore } from '../../stores/traceStore';

// PingPlotter-inspired hop color palette
const HOP_COLORS = [
  '#22c55e', '#3b82f6', '#f59e0b', '#ef4444', '#a855f7',
  '#06b6d4', '#f97316', '#ec4899', '#14b8a6', '#8b5cf6',
  '#84cc16', '#e11d48', '#0ea5e9', '#d946ef', '#64748b',
  '#fb923c', '#34d399', '#818cf8', '#fbbf24', '#f472b6',
];

interface LatencyTimelineChartProps {
  agentId: string;
  targetId: string;
  selectedHop: number | null;
}

function buildOpts(
  width: number,
  height: number,
  hopNumbers: number[],
  selectedHop: number | null,
): uPlot.Options {
  const series: uPlot.Series[] = [
    { label: 'Time' },
    ...hopNumbers.map((hopNum, i) => ({
      label: `Hop ${hopNum}`,
      stroke: HOP_COLORS[i % HOP_COLORS.length],
      width: selectedHop === hopNum ? 2.5 : 1.2,
      alpha: selectedHop != null && selectedHop !== hopNum ? 0.15 : 1,
      spanGaps: false,
      points: { show: false },
    })),
  ];

  return {
    width,
    height,
    series,
    scales: {
      x: { time: true },
      y: {
        auto: true,
        range: (_u: uPlot, _min: number, max: number) => [0, Math.max(max * 1.15, 1)] as uPlot.Range.MinMax,
      },
    },
    axes: [
      {
        stroke: '#94a3b8',
        grid: { stroke: 'rgba(148, 163, 184, 0.08)', width: 1 },
        ticks: { stroke: 'rgba(148, 163, 184, 0.15)', width: 1 },
        font: '10px "Inter", system-ui, sans-serif',
        gap: 6,
      },
      {
        stroke: '#94a3b8',
        grid: { stroke: 'rgba(148, 163, 184, 0.08)', width: 1 },
        ticks: { stroke: 'rgba(148, 163, 184, 0.15)', width: 1 },
        font: '10px "Inter", system-ui, sans-serif',
        label: 'RTT (ms)',
        labelFont: '11px "Inter", system-ui, sans-serif',
        size: 55,
        gap: 8,
        values: (_u: uPlot, vals: number[]) => vals.map((v) => v < 1 ? v.toFixed(2) : v < 10 ? v.toFixed(1) : Math.round(v).toString()),
      },
    ],
    cursor: {
      drag: { x: true, y: false, setScale: true },
      points: { size: 6, fill: '#fff' },
    },
    legend: { show: false },
    padding: [12, 8, 0, 0],
  };
}

export function LatencyTimelineChart({ agentId, targetId, selectedHop }: LatencyTimelineChartProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const chartRef = useRef<uPlot | null>(null);
  const prevHopsRef = useRef<string>('');

  const roundCount = useTraceStore(
    useCallback(
      (s) => s.getRoundCount(agentId, targetId),
      [agentId, targetId],
    ),
  );

  const timeSeries = useMemo(() => {
    return useTraceStore.getState().getTimeSeries(agentId, targetId);
  }, [agentId, targetId, roundCount]);

  // Build plot data from time series
  const plotData = useMemo(() => {
    if (!timeSeries || timeSeries.data.length < 2) return null;
    return timeSeries.data.map((arr) => Array.from(arr)) as uPlot.AlignedData;
  }, [timeSeries]);

  // Recreate chart when hop structure changes or selectedHop changes
  useEffect(() => {
    if (!containerRef.current || !timeSeries || !plotData) return;

    const hopsKey = timeSeries.hopNumbers.join(',');
    const structureChanged = hopsKey !== prevHopsRef.current;
    prevHopsRef.current = hopsKey;

    if (chartRef.current && !structureChanged) {
      chartRef.current.setData(plotData);
      return;
    }

    // Destroy old chart if structure changed
    if (chartRef.current) {
      chartRef.current.destroy();
      chartRef.current = null;
    }

    const opts = buildOpts(
      containerRef.current.clientWidth,
      containerRef.current.clientHeight,
      timeSeries.hopNumbers,
      selectedHop,
    );

    chartRef.current = new uPlot(opts, plotData, containerRef.current);

    return () => {
      chartRef.current?.destroy();
      chartRef.current = null;
    };
  }, [plotData, timeSeries, selectedHop]);

  // Handle resize
  useEffect(() => {
    if (!containerRef.current) return;
    const observer = new ResizeObserver((entries) => {
      const entry = entries[0];
      if (entry && chartRef.current) {
        chartRef.current.setSize({
          width: entry.contentRect.width,
          height: entry.contentRect.height,
        });
      }
    });
    observer.observe(containerRef.current);
    return () => observer.disconnect();
  }, []);

  if (!timeSeries) {
    return (
      <div className="flex items-center justify-center h-full text-[var(--text-secondary)] text-sm">
        Waiting for trace data...
      </div>
    );
  }

  return (
    <div className="relative w-full h-full">
      {/* Legend overlay */}
      <div className="absolute top-1 right-2 z-10 flex flex-wrap gap-x-3 gap-y-0.5 bg-[var(--bg-surface)]/80 backdrop-blur-sm rounded px-2 py-1 max-w-[60%]">
        {timeSeries.hopNumbers.map((hopNum, i) => (
          <div
            key={hopNum}
            className={`flex items-center gap-1 text-[10px] ${
              selectedHop != null && selectedHop !== hopNum ? 'opacity-30' : ''
            }`}
          >
            <div
              className="w-2.5 h-0.5 rounded-full"
              style={{ backgroundColor: HOP_COLORS[i % HOP_COLORS.length] }}
            />
            <span className="text-[var(--text-secondary)]">{hopNum}</span>
          </div>
        ))}
      </div>
      <div ref={containerRef} className="w-full h-full" />
    </div>
  );
}
