import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { useParams } from 'react-router-dom';
import { Download, Camera, Share2 } from 'lucide-react';
import { useWS } from '../../ws/WebSocketProvider';
import { useTraceStore } from '../../stores/traceStore';
import { useUIStore } from '../../stores/uiStore';
import { useTarget } from '../../api/queries/useTargets';
import { HopTable } from './HopTable';
import { HopDetailPanel } from './HopDetailPanel';
import { LatencyTimelineChart } from './LatencyTimelineChart';
import { StripChart } from './StripChart';
import { LossChart } from './LossChart';
import { JitterChart } from './JitterChart';
import { ShareDialog } from './ShareDialog';

type ChartMode = 'timeline' | 'strip' | 'loss' | 'jitter';

const CHART_MODES: { key: ChartMode; label: string }[] = [
  { key: 'timeline', label: 'Timeline' },
  { key: 'strip', label: 'Strip' },
  { key: 'loss', label: 'Loss' },
  { key: 'jitter', label: 'Jitter' },
];

/** Capture all canvases inside a container and composite them into a single PNG download. */
function exportChartImage(container: HTMLElement, filename: string) {
  const canvases = container.querySelectorAll('canvas');
  if (canvases.length === 0) return;

  // If single canvas (strip chart), just export directly
  if (canvases.length === 1) {
    canvases[0].toBlob((blob) => {
      if (!blob) return;
      triggerDownload(blob, filename);
    }, 'image/png');
    return;
  }

  // Multiple canvases (uPlot) — composite onto a single offscreen canvas
  const rect = container.getBoundingClientRect();
  const dpr = window.devicePixelRatio || 1;
  const offscreen = document.createElement('canvas');
  offscreen.width = rect.width * dpr;
  offscreen.height = rect.height * dpr;
  const ctx = offscreen.getContext('2d');
  if (!ctx) return;

  // Fill with the surface background color
  ctx.fillStyle = getComputedStyle(container).backgroundColor || '#1e293b';
  ctx.fillRect(0, 0, offscreen.width, offscreen.height);

  for (const canvas of canvases) {
    const cr = canvas.getBoundingClientRect();
    const x = (cr.left - rect.left) * dpr;
    const y = (cr.top - rect.top) * dpr;
    ctx.drawImage(canvas, x, y, cr.width * dpr, cr.height * dpr);
  }

  offscreen.toBlob((blob) => {
    if (!blob) return;
    triggerDownload(blob, filename);
  }, 'image/png');
}

function triggerDownload(blob: Blob, filename: string) {
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = filename;
  document.body.appendChild(a);
  a.click();
  document.body.removeChild(a);
  URL.revokeObjectURL(url);
}

export function TracePage() {
  const { agentId, targetId } = useParams<{ agentId: string; targetId: string }>();
  const { subscribe, unsubscribe } = useWS();
  const [chartMode, setChartMode] = useState<ChartMode>('timeline');
  const [showShareDialog, setShowShareDialog] = useState(false);
  const { selectedHop, selectHop, detailPanelOpen, closeDetailPanel } = useUIStore();
  const { data: target } = useTarget(targetId ?? '');
  const chartContainerRef = useRef<HTMLDivElement>(null);

  // Subscribe only to a primitive (number) to avoid useSyncExternalStore tear loops
  const roundCount = useTraceStore(
    useCallback(
      (s) => (agentId && targetId ? s.getRoundCount(agentId, targetId) : 0),
      [agentId, targetId],
    ),
  );

  // Derive complex data outside the subscription via useMemo
  const hops = useMemo(() => {
    if (!agentId || !targetId) return [];
    return useTraceStore.getState().getHopsArray(agentId, targetId);
  }, [agentId, targetId, roundCount]);

  // Subscribe to live trace data
  useEffect(() => {
    if (!agentId || !targetId) return;
    subscribe([targetId]);
    return () => unsubscribe([targetId]);
  }, [agentId, targetId, subscribe, unsubscribe]);

  // Init trace in store
  useEffect(() => {
    if (agentId && targetId) {
      useTraceStore.getState().initTrace(agentId, targetId);
    }
  }, [agentId, targetId]);

  if (!agentId || !targetId) {
    return <div className="text-[var(--text-secondary)]">No trace selected</div>;
  }

  const displayName = target?.display_name ?? target?.address ?? targetId;

  return (
    <div className="h-full flex flex-col gap-2">
      {/* Toolbar */}
      <div className="flex items-center gap-3 px-3 py-1.5 bg-[var(--bg-surface)] border border-[var(--border-default)] rounded">
        <div className="flex items-center gap-2">
          <span className="text-xs text-[var(--text-secondary)]">Target:</span>
          <span className="text-sm font-semibold">{displayName}</span>
        </div>
        <div className="h-4 w-px bg-[var(--border-default)]" />
        <div className="flex items-center gap-3 text-xs text-[var(--text-secondary)]">
          <span>{roundCount} rounds</span>
          <span>{hops.length} hops</span>
        </div>
        <div className="ml-auto flex items-center gap-1">
          {CHART_MODES.map(({ key, label }) => (
            <button
              key={key}
              onClick={() => setChartMode(key)}
              className={`px-2.5 py-1 text-xs rounded transition-colors ${
                chartMode === key
                  ? 'bg-[var(--accent)] text-white'
                  : 'text-[var(--text-secondary)] hover:bg-[var(--bg-elevated)] hover:text-[var(--text-primary)]'
              }`}
            >
              {label}
            </button>
          ))}
          <div className="h-4 w-px bg-[var(--border-default)] mx-1" />
          <button
            onClick={() => {
              if (chartContainerRef.current) {
                const ts = new Date().toISOString().replace(/[:.]/g, '-').slice(0, 19);
                exportChartImage(chartContainerRef.current, `trace-${chartMode}-${ts}.png`);
              }
            }}
            title="Save chart as PNG"
            className="flex items-center gap-1 px-2 py-1 text-xs rounded text-[var(--text-secondary)] hover:bg-[var(--bg-elevated)] hover:text-[var(--text-primary)] transition-colors"
          >
            <Camera className="w-3.5 h-3.5" />
          </button>
          <button
            onClick={() => {
              window.open(`/api/v1/export/csv/${targetId}`, '_blank');
            }}
            title="Export session data as CSV"
            className="flex items-center gap-1 px-2 py-1 text-xs rounded text-[var(--text-secondary)] hover:bg-[var(--bg-elevated)] hover:text-[var(--text-primary)] transition-colors"
          >
            <Download className="w-3.5 h-3.5" />
          </button>
          <button
            onClick={() => setShowShareDialog(true)}
            title="Share this trace"
            className="flex items-center gap-1 px-2 py-1 text-xs rounded text-[var(--text-secondary)] hover:bg-[var(--bg-elevated)] hover:text-[var(--text-primary)] transition-colors"
          >
            <Share2 className="w-3.5 h-3.5" />
          </button>
        </div>
      </div>

      {/* Main content: Hop table + Chart */}
      <div className="flex-1 flex gap-2 min-h-0">
        {/* Hop Table */}
        <div className="w-[42%] bg-[var(--bg-surface)] border border-[var(--border-default)] rounded overflow-auto">
          <HopTable hops={hops} selectedHop={selectedHop} onHopClick={selectHop} />
        </div>

        {/* Chart Area */}
        <div ref={chartContainerRef} className="flex-1 bg-[var(--bg-surface)] border border-[var(--border-default)] rounded overflow-hidden">
          <ChartPanel
            agentId={agentId}
            targetId={targetId}
            chartMode={chartMode}
            selectedHop={selectedHop}
          />
        </div>
      </div>

      {/* Hop Detail Panel */}
      {detailPanelOpen && selectedHop !== null && (
        <HopDetailPanel
          hopNumber={selectedHop}
          hops={hops}
          onClose={closeDetailPanel}
        />
      )}

      {/* Share Dialog */}
      {showShareDialog && targetId && (
        <ShareDialog targetId={targetId} onClose={() => setShowShareDialog(false)} />
      )}
    </div>
  );
}

function ChartPanel({
  agentId,
  targetId,
  chartMode,
  selectedHop,
}: {
  agentId: string;
  targetId: string;
  chartMode: ChartMode;
  selectedHop: number | null;
}) {
  switch (chartMode) {
    case 'timeline':
      return <LatencyTimelineChart agentId={agentId} targetId={targetId} selectedHop={selectedHop} />;
    case 'strip':
      return <StripChart agentId={agentId} targetId={targetId} selectedHop={selectedHop} />;
    case 'loss':
      return <LossChart agentId={agentId} targetId={targetId} selectedHop={selectedHop} />;
    case 'jitter':
      return <JitterChart agentId={agentId} targetId={targetId} selectedHop={selectedHop} />;
  }
}
