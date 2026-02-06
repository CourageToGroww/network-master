import { useCallback, useEffect, useMemo, useRef } from 'react';
import uPlot from 'uplot';
import 'uplot/dist/uPlot.min.css';
import { useTraceStore } from '../../stores/traceStore';

const LOSS_COLORS = [
  '#ef4444', '#f97316', '#f59e0b', '#eab308', '#84cc16',
  '#22c55e', '#14b8a6', '#06b6d4', '#3b82f6', '#8b5cf6',
  '#a855f7', '#d946ef', '#ec4899', '#f43f5e', '#64748b',
];

interface LossChartProps {
  agentId: string;
  targetId: string;
  selectedHop: number | null;
}

export function LossChart({ agentId, targetId, selectedHop }: LossChartProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const chartRef = useRef<uPlot | null>(null);
  const prevHopsRef = useRef<string>('');

  const roundCount = useTraceStore(
    useCallback(
      (s) => s.getRoundCount(agentId, targetId),
      [agentId, targetId],
    ),
  );

  const lossSeries = useMemo(() => {
    return useTraceStore.getState().getLossTimeSeries(agentId, targetId);
  }, [agentId, targetId, roundCount]);

  const plotData = useMemo(() => {
    if (!lossSeries || lossSeries.data.length < 2) return null;
    return lossSeries.data.map((arr) => Array.from(arr)) as uPlot.AlignedData;
  }, [lossSeries]);

  useEffect(() => {
    if (!containerRef.current || !lossSeries || !plotData) return;

    const hopsKey = lossSeries.hopNumbers.join(',');
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
      ...lossSeries.hopNumbers.map((hopNum, i) => ({
        label: `Hop ${hopNum}`,
        stroke: LOSS_COLORS[i % LOSS_COLORS.length],
        fill: LOSS_COLORS[i % LOSS_COLORS.length] + '18',
        width: selectedHop === hopNum ? 2.5 : 1,
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
          auto: false,
          range: [0, 100] as uPlot.Range.MinMax,
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
          label: 'Packet Loss (%)',
          labelFont: '11px "Inter", system-ui, sans-serif',
          size: 55,
          gap: 8,
          values: (_u: uPlot, vals: number[]) => vals.map((v) => `${v.toFixed(0)}%`),
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
  }, [plotData, lossSeries, selectedHop]);

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

  if (!lossSeries) {
    return (
      <div className="flex items-center justify-center h-full text-[var(--text-secondary)] text-sm">
        Waiting for trace data...
      </div>
    );
  }

  return (
    <div className="relative w-full h-full">
      <div className="absolute top-1 right-2 z-10 flex flex-wrap gap-x-3 gap-y-0.5 bg-[var(--bg-surface)]/80 backdrop-blur-sm rounded px-2 py-1 max-w-[60%]">
        {lossSeries.hopNumbers.map((hopNum, i) => (
          <div
            key={hopNum}
            className={`flex items-center gap-1 text-[10px] ${
              selectedHop != null && selectedHop !== hopNum ? 'opacity-30' : ''
            }`}
          >
            <div
              className="w-2.5 h-0.5 rounded-full"
              style={{ backgroundColor: LOSS_COLORS[i % LOSS_COLORS.length] }}
            />
            <span className="text-[var(--text-secondary)]">{hopNum}</span>
          </div>
        ))}
      </div>
      <div ref={containerRef} className="w-full h-full" />
    </div>
  );
}
