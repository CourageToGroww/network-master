import { X } from 'lucide-react';
import type { HopRealtimeData } from '../../types';

interface HopDetailPanelProps {
  hopNumber: number;
  hops: HopRealtimeData[];
  onClose: () => void;
}

function mosLabel(mos: number): string {
  if (mos >= 4.3) return 'Excellent';
  if (mos >= 4.0) return 'Good';
  if (mos >= 3.6) return 'Fair';
  if (mos >= 3.1) return 'Poor';
  if (mos >= 2.6) return 'Bad';
  return 'Critical';
}

function mosColor(mos: number): string {
  if (mos >= 4.0) return '#22c55e';
  if (mos >= 3.5) return '#a3e635';
  if (mos >= 3.0) return '#facc15';
  if (mos >= 2.5) return '#f59e0b';
  return '#ef4444';
}

export function HopDetailPanel({ hopNumber, hops, onClose }: HopDetailPanelProps) {
  const hop = hops.find((h) => h.hopNumber === hopNumber);
  if (!hop) return null;

  const statItems = [
    { label: 'IP Address', value: hop.ip ?? 'N/A', mono: true },
    { label: 'Hostname', value: hop.hostname ?? 'N/A', mono: true },
    { label: 'Avg Latency', value: `${hop.avgMs.toFixed(2)} ms`, mono: true },
    { label: 'Packet Loss', value: `${hop.lossPct.toFixed(1)}%`, mono: true },
    { label: 'Best RTT', value: `${hop.bestMs.toFixed(2)} ms`, mono: true },
    { label: 'Worst RTT', value: `${hop.worstMs.toFixed(2)} ms`, mono: true },
    { label: 'Jitter', value: `${hop.jitterMs.toFixed(2)} ms`, mono: true },
    { label: 'Samples', value: `${hop.sent} sent / ${hop.recv} recv`, mono: true },
  ];

  return (
    <div className="bg-[var(--bg-surface)] border border-[var(--border-default)] rounded p-4">
      <div className="flex items-center justify-between mb-3">
        <div className="flex items-center gap-3">
          <h3 className="text-sm font-semibold">
            Hop {hop.hopNumber}: {hop.hostname ?? hop.ip ?? 'Unknown'}
          </h3>
          <div
            className="flex items-center gap-1.5 px-2 py-0.5 rounded text-xs font-medium"
            style={{
              backgroundColor: mosColor(hop.mos) + '20',
              color: mosColor(hop.mos),
            }}
          >
            MOS {hop.mos.toFixed(1)} - {mosLabel(hop.mos)}
          </div>
        </div>
        <button onClick={onClose} className="text-[var(--text-secondary)] hover:text-[var(--text-primary)]">
          <X className="w-4 h-4" />
        </button>
      </div>

      <div className="grid grid-cols-4 gap-4 text-xs">
        {statItems.map(({ label, value, mono }) => (
          <div key={label}>
            <div className="text-[var(--text-secondary)]">{label}</div>
            <div className={`mt-0.5 ${mono ? 'font-mono' : ''}`}>{value}</div>
          </div>
        ))}
      </div>
    </div>
  );
}
