import { useCallback, useEffect, useMemo, useRef } from 'react';
import uPlot from 'uplot';
import 'uplot/dist/uPlot.min.css';
import { useTraceStore } from '../../stores/traceStore';

const JITTER_COLORS = [
  '#8b5cf6', '#06b6d4', '#22c55e', '#f59e0b', '#ef4444',
  '#a855f7', '#14b8a6', '#3b82f6', '#f97316', '#ec4899',
  '#84cc16', '#e11d48', '#0ea5e9', '#d946ef', '#64748b',
];

interface JitterChartProps {
  agentId: string;
  targetId: string;
  selectedHop: number | null;
}

export function JitterChart({ agentId, targetId, selectedHop }: JitterChartProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const chartRef = useRef<uPlot | null>(null);
  const prevHopsRef = useRef<string>('');

  const roundCount = useTraceStore(
    useCallback(
      (s) => s.getRoundCount(agentId, targetId),
      [agentId, targetId],
    ),
  );

  const jitterSeries = useMemo(() => {
    return useTraceStore.getState().getJitterTimeSeries(agentId, targetId);
  }, [agentId, targetId, roundCount]);

  const plotData = useMemo(() => {
    if (!jitterSeries || jitterSeries.data.length < 2) return null;
    return jitterSeries.data.map((arr) => Array.from(arr)) as uPlot.AlignedData;
  }, [jitterSeries]);

  useEffect(() => {
    if (!containerRef.current || !jitterSeries || !plotData) return;

    const hopsKey = jitterSeries.hopNumbers.join(',');
    const structureChanged = hopsKey !== prevHopsRef.current;
    prevHopsRef.current = hopsKey;

    if (chartRef.current && !structureChanged) {
      chartRef.current.setData(plotData);
      return;
    }

    if (chartRef.current) {
      chartRef.current.destroy();
      chartRef.current = null;
    }

    const series: uPlot.Series[] = [
      { label: 'Time' },
      ...jitterSeries.hopNumbers.map((hopNum, i) => ({
        label: `Hop ${hopNum}`,
        stroke: JITTER_COLORS[i % JITTER_COLORS.length],
        width: selectedHop === hopNum ? 2.5 : 1.2,
        alpha: selectedHop != null && selectedHop !== hopNum ? 0.15 : 1,
        spanGaps: true,
        points: { show: false },
      })),
    ];

    const opts: uPlot.Options = {
      width: containerRef.current.clientWidth,
      height: containerRef.current.clientHeight,
      series,
      scales: {
        x: { time: true },
        y: {
          auto: true,
          range: (_u: uPlot, _min: number, max: number) => [0, Math.max(max * 1.15, 0.1)] as uPlot.Range.MinMax,
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
          label: 'Jitter (ms)',
          labelFont: '11px "Inter", system-ui, sans-serif',
          size: 55,
          gap: 8,
          values: (_u: uPlot, vals: number[]) => vals.map((v) => v < 1 ? v.toFixed(2) : v.toFixed(1)),
        },
      ],
      cursor: {
        drag: { x: true, y: false, setScale: true },
        points: { size: 6, fill: '#fff' },
      },
      legend: { show: false },
      padding: [12, 8, 0, 0],
    };

    chartRef.current = new uPlot(opts, plotData, containerRef.current);

    return () => {
      chartRef.current?.destroy();
      chartRef.current = null;
    };
  }, [plotData, jitterSeries, selectedHop]);

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

  if (!jitterSeries) {
    return (
      <div className="flex items-center justify-center h-full text-[var(--text-secondary)] text-sm">
        Waiting for trace data...
      </div>
    );
  }

  return (
    <div className="relative w-full h-full">
      <div className="absolute top-1 right-2 z-10 flex flex-wrap gap-x-3 gap-y-0.5 bg-[var(--bg-surface)]/80 backdrop-blur-sm rounded px-2 py-1 max-w-[60%]">
        {jitterSeries.hopNumbers.map((hopNum, i) => (
          <div
            key={hopNum}
            className={`flex items-center gap-1 text-[10px] ${
              selectedHop != null && selectedHop !== hopNum ? 'opacity-30' : ''
            }`}
          >
            <div
              className="w-2.5 h-0.5 rounded-full"
              style={{ backgroundColor: JITTER_COLORS[i % JITTER_COLORS.length] }}
            />
            <span className="text-[var(--text-secondary)]">{hopNum}</span>
          </div>
        ))}
      </div>
      <div ref={containerRef} className="w-full h-full" />
    </div>
  );
}
