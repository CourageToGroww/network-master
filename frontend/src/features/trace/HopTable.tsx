import type { HopRealtimeData } from '../../types';

interface HopTableProps {
  hops: HopRealtimeData[];
  selectedHop: number | null;
  onHopClick: (hopNumber: number) => void;
}

function formatMs(ms: number): string {
  if (ms < 0.01) return '<0.01';
  if (ms < 1) return ms.toFixed(2);
  if (ms < 10) return ms.toFixed(1);
  return Math.round(ms).toString();
}

function lossColor(pct: number): string {
  if (pct > 20) return 'text-[#ef4444] font-semibold';
  if (pct > 5) return 'text-[#f59e0b]';
  if (pct > 0) return 'text-[#fbbf24]';
  return '';
}

function mosColor(mos: number): string {
  if (mos >= 4.0) return 'text-[#22c55e]';
  if (mos >= 3.5) return 'text-[#a3e635]';
  if (mos >= 3.0) return 'text-[#facc15]';
  if (mos >= 2.5) return 'text-[#f59e0b]';
  return 'text-[#ef4444]';
}

export function HopTable({ hops, selectedHop, onHopClick }: HopTableProps) {
  return (
    <table className="w-full text-xs">
      <thead className="sticky top-0 bg-[var(--bg-surface)] border-b border-[var(--border-default)] z-10">
        <tr className="text-[var(--text-secondary)]">
          <th className="px-2 py-1.5 text-left w-8">#</th>
          <th className="px-2 py-1.5 text-left">IP</th>
          <th className="px-2 py-1.5 text-left">Hostname</th>
          <th className="px-2 py-1.5 text-right w-14">Loss%</th>
          <th className="px-2 py-1.5 text-right w-12">Sent</th>
          <th className="px-2 py-1.5 text-right w-12">Recv</th>
          <th className="px-2 py-1.5 text-right w-14">Best</th>
          <th className="px-2 py-1.5 text-right w-14">Avg</th>
          <th className="px-2 py-1.5 text-right w-14">Worst</th>
          <th className="px-2 py-1.5 text-right w-14">Last</th>
          <th className="px-2 py-1.5 text-right w-14">Jitter</th>
          <th className="px-2 py-1.5 text-right w-12">MOS</th>
        </tr>
      </thead>
      <tbody>
        {hops.map((hop) => (
          <tr
            key={hop.hopNumber}
            onClick={() => onHopClick(hop.hopNumber)}
            className={`cursor-pointer hover:bg-[var(--bg-elevated)] transition-colors ${
              selectedHop === hop.hopNumber ? 'bg-[var(--bg-elevated)]' : ''
            }`}
          >
            <td className="px-2 py-1 font-mono text-[var(--text-secondary)]">{hop.hopNumber}</td>
            <td className="px-2 py-1 font-mono">{hop.ip ?? '*'}</td>
            <td className="px-2 py-1 truncate max-w-[120px]">{hop.hostname ?? '-'}</td>
            <td className={`px-2 py-1 text-right font-mono ${lossColor(hop.lossPct)}`}>
              {hop.lossPct.toFixed(1)}%
            </td>
            <td className="px-2 py-1 text-right font-mono text-[var(--text-secondary)]">{hop.sent}</td>
            <td className="px-2 py-1 text-right font-mono text-[var(--text-secondary)]">{hop.recv}</td>
            <td className="px-2 py-1 text-right font-mono">{formatMs(hop.bestMs)}</td>
            <td className="px-2 py-1 text-right font-mono">{formatMs(hop.avgMs)}</td>
            <td className="px-2 py-1 text-right font-mono">{formatMs(hop.worstMs)}</td>
            <td className="px-2 py-1 text-right font-mono">{formatMs(hop.lastMs)}</td>
            <td className="px-2 py-1 text-right font-mono">{formatMs(hop.jitterMs)}</td>
            <td className={`px-2 py-1 text-right font-mono ${mosColor(hop.mos)}`}>
              {hop.mos.toFixed(1)}
            </td>
          </tr>
        ))}
        {hops.length === 0 && (
          <tr>
            <td colSpan={12} className="px-4 py-8 text-center text-[var(--text-secondary)]">
              No hop data yet. Waiting for agent...
            </td>
          </tr>
        )}
      </tbody>
    </table>
  );
}
