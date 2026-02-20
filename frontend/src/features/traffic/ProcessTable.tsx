import { useState } from 'react';
import { ChevronDown, ChevronRight } from 'lucide-react';
import type { ProcessTrafficSummary } from '../../types';

function formatBytes(bytesPerSec: number): string {
  if (bytesPerSec < 1024) return `${bytesPerSec.toFixed(0)} B/s`;
  if (bytesPerSec < 1024 * 1024) return `${(bytesPerSec / 1024).toFixed(1)} KB/s`;
  return `${(bytesPerSec / 1024 / 1024).toFixed(2)} MB/s`;
}

type SortKey = 'process_name' | 'pid' | 'bytes_in' | 'bytes_out' | 'connections';
type SortDir = 'asc' | 'desc';

interface Props {
  processes: ProcessTrafficSummary[];
}

export function ProcessTable({ processes }: Props) {
  const [sortKey, setSortKey] = useState<SortKey>('bytes_in');
  const [sortDir, setSortDir] = useState<SortDir>('desc');
  const [expandedPid, setExpandedPid] = useState<number | null>(null);

  const handleSort = (key: SortKey) => {
    if (sortKey === key) {
      setSortDir(sortDir === 'asc' ? 'desc' : 'asc');
    } else {
      setSortKey(key);
      setSortDir('desc');
    }
  };

  const sorted = [...processes].sort((a, b) => {
    const dir = sortDir === 'asc' ? 1 : -1;
    switch (sortKey) {
      case 'process_name': return dir * a.process_name.localeCompare(b.process_name);
      case 'pid': return dir * (a.pid - b.pid);
      case 'bytes_in': return dir * (a.bytes_in_per_sec - b.bytes_in_per_sec);
      case 'bytes_out': return dir * (a.bytes_out_per_sec - b.bytes_out_per_sec);
      case 'connections': return dir * (a.active_connections - b.active_connections);
      default: return 0;
    }
  });

  const SortHeader = ({ label, column }: { label: string; column: SortKey }) => (
    <th
      className="px-3 py-2 text-left text-xs font-medium text-[var(--text-secondary)] cursor-pointer select-none hover:text-[var(--text-primary)]"
      onClick={() => handleSort(column)}
    >
      {label}
      {sortKey === column && (
        <span className="ml-1">{sortDir === 'asc' ? '\u25B2' : '\u25BC'}</span>
      )}
    </th>
  );

  // Compute max bandwidth for bar widths
  const maxBandwidth = Math.max(
    1,
    ...processes.map((p) => Math.max(p.bytes_in_per_sec, p.bytes_out_per_sec))
  );

  return (
    <div className="bg-[var(--bg-surface)] border border-[var(--border-default)] rounded-lg overflow-hidden">
      <table className="w-full text-sm">
        <thead className="bg-[var(--bg-elevated)]">
          <tr>
            <th className="w-8" />
            <SortHeader label="Process" column="process_name" />
            <SortHeader label="PID" column="pid" />
            <SortHeader label="Download" column="bytes_in" />
            <SortHeader label="Upload" column="bytes_out" />
            <SortHeader label="Connections" column="connections" />
          </tr>
        </thead>
        <tbody className="divide-y divide-[var(--border-default)]">
          {sorted.map((proc) => {
            const isExpanded = expandedPid === proc.pid;
            return (
              <ProcessRow
                key={proc.pid}
                proc={proc}
                isExpanded={isExpanded}
                maxBandwidth={maxBandwidth}
                onToggle={() => setExpandedPid(isExpanded ? null : proc.pid)}
              />
            );
          })}
          {sorted.length === 0 && (
            <tr>
              <td colSpan={6} className="px-3 py-8 text-center text-[var(--text-secondary)]">
                No active network processes
              </td>
            </tr>
          )}
        </tbody>
      </table>
    </div>
  );
}

function ProcessRow({
  proc,
  isExpanded,
  maxBandwidth,
  onToggle,
}: {
  proc: ProcessTrafficSummary;
  isExpanded: boolean;
  maxBandwidth: number;
  onToggle: () => void;
}) {
  const hasEndpoints = proc.top_remote_endpoints.length > 0;
  const inPct = (proc.bytes_in_per_sec / maxBandwidth) * 100;
  const outPct = (proc.bytes_out_per_sec / maxBandwidth) * 100;

  return (
    <>
      <tr
        className={`hover:bg-[var(--bg-elevated)] transition-colors ${
          hasEndpoints ? 'cursor-pointer' : ''
        }`}
        onClick={hasEndpoints ? onToggle : undefined}
      >
        <td className="px-2 py-2 text-center">
          {hasEndpoints && (
            isExpanded ? (
              <ChevronDown className="w-3.5 h-3.5 text-[var(--text-secondary)]" />
            ) : (
              <ChevronRight className="w-3.5 h-3.5 text-[var(--text-secondary)]" />
            )
          )}
        </td>
        <td className="px-3 py-2">
          <div className="font-medium">{proc.process_name}</div>
          {proc.exe_path && (
            <div className="text-xs text-[var(--text-secondary)] truncate max-w-[300px]">
              {proc.exe_path}
            </div>
          )}
        </td>
        <td className="px-3 py-2 text-[var(--text-secondary)] tabular-nums">{proc.pid}</td>
        <td className="px-3 py-2">
          <div className="flex items-center gap-2">
            <div className="w-24 h-2 bg-[var(--bg-elevated)] rounded-full overflow-hidden">
              <div
                className="h-full bg-green-500 rounded-full"
                style={{ width: `${Math.max(inPct, 1)}%` }}
              />
            </div>
            <span className="tabular-nums text-green-400 min-w-[80px]">
              {formatBytes(proc.bytes_in_per_sec)}
            </span>
          </div>
        </td>
        <td className="px-3 py-2">
          <div className="flex items-center gap-2">
            <div className="w-24 h-2 bg-[var(--bg-elevated)] rounded-full overflow-hidden">
              <div
                className="h-full bg-blue-500 rounded-full"
                style={{ width: `${Math.max(outPct, 1)}%` }}
              />
            </div>
            <span className="tabular-nums text-blue-400 min-w-[80px]">
              {formatBytes(proc.bytes_out_per_sec)}
            </span>
          </div>
        </td>
        <td className="px-3 py-2 tabular-nums">{proc.active_connections}</td>
      </tr>
      {isExpanded && hasEndpoints && (
        <tr>
          <td colSpan={6} className="bg-[var(--bg-elevated)] px-6 py-3">
            <div className="text-xs font-medium text-[var(--text-secondary)] mb-2">
              Remote Endpoints
            </div>
            <table className="w-full text-xs">
              <thead>
                <tr className="text-[var(--text-secondary)]">
                  <th className="text-left py-1 pr-4">Remote Address</th>
                  <th className="text-left py-1 pr-4">Port</th>
                  <th className="text-left py-1 pr-4">Protocol</th>
                  <th className="text-right py-1 pr-4">Download</th>
                  <th className="text-right py-1">Upload</th>
                </tr>
              </thead>
              <tbody>
                {proc.top_remote_endpoints.map((ep, i) => (
                  <tr key={i} className="text-[var(--text-primary)]">
                    <td className="py-1 pr-4 tabular-nums">{ep.remote_addr}</td>
                    <td className="py-1 pr-4 tabular-nums">{ep.remote_port}</td>
                    <td className="py-1 pr-4 uppercase">{ep.protocol}</td>
                    <td className="py-1 pr-4 text-right tabular-nums text-green-400">
                      {formatBytes(ep.bytes_in_per_sec)}
                    </td>
                    <td className="py-1 text-right tabular-nums text-blue-400">
                      {formatBytes(ep.bytes_out_per_sec)}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </td>
        </tr>
      )}
    </>
  );
}
