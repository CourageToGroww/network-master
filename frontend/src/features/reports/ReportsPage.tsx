import { useMemo, useState } from 'react';
import { Download, FileText, BarChart3 } from 'lucide-react';
import { useQuery } from '@tanstack/react-query';
import { useAgents } from '../../api/queries/useAgents';
import { useTargets } from '../../api/queries/useTargets';
import { api } from '../../api/client';
import type { TraceSession } from '../../types';

export function ReportsPage() {
  const { data: agents } = useAgents();
  const [selectedAgentId, setSelectedAgentId] = useState<string>('');
  const { data: targets } = useTargets(selectedAgentId);
  const [selectedTargetId, setSelectedTargetId] = useState<string>('');

  const { data: sessions } = useQuery({
    queryKey: ['sessions', selectedTargetId],
    queryFn: () => api.get<TraceSession[]>(`/targets/${selectedTargetId}/sessions`),
    enabled: !!selectedTargetId,
  });

  const handleExportCSV = (sessionId: string) => {
    const url = `/api/v1/export/csv/${sessionId}`;
    window.open(url, '_blank');
  };

  // Compute summary statistics from sessions
  const summary = useMemo(() => {
    if (!sessions || sessions.length === 0) return null;

    const activeSessions = sessions.filter((s) => !s.ended_at);
    const totalSamples = sessions.reduce((acc, s) => acc + s.sample_count, 0);

    // Calculate total monitoring duration
    let totalDurationMs = 0;
    for (const s of sessions) {
      const start = new Date(s.started_at).getTime();
      const end = s.ended_at ? new Date(s.ended_at).getTime() : Date.now();
      totalDurationMs += end - start;
    }

    // Earliest and latest
    const earliest = sessions.reduce(
      (min, s) => (s.started_at < min ? s.started_at : min),
      sessions[0].started_at,
    );
    const latest = sessions.reduce((max, s) => {
      const t = s.ended_at ?? s.started_at;
      return t > max ? t : max;
    }, sessions[0].started_at);

    return {
      totalSessions: sessions.length,
      activeSessions: activeSessions.length,
      totalSamples,
      totalDurationMs,
      earliest,
      latest,
    };
  }, [sessions]);

  const selectedTarget = targets?.find((t) => t.id === selectedTargetId);

  return (
    <div className="space-y-6">
      <div className="flex items-center gap-2">
        <FileText className="w-5 h-5 text-[var(--accent)]" />
        <h1 className="text-lg font-semibold">Reports & Export</h1>
      </div>

      {/* Filters */}
      <div className="bg-[var(--bg-surface)] border border-[var(--border-default)] rounded-lg p-4">
        <div className="grid grid-cols-2 gap-4">
          <div>
            <label className="block text-xs text-[var(--text-secondary)] mb-1.5">Agent</label>
            <select
              value={selectedAgentId}
              onChange={(e) => { setSelectedAgentId(e.target.value); setSelectedTargetId(''); }}
              className="w-full px-3 py-2 text-sm rounded bg-[var(--bg-primary)] border border-[var(--border-default)] focus:border-[var(--accent)] outline-none"
            >
              <option value="">Select an agent...</option>
              {agents?.map((a) => (
                <option key={a.id} value={a.id}>{a.name} ({a.hostname ?? a.ip_address ?? 'unknown'})</option>
              ))}
            </select>
          </div>
          <div>
            <label className="block text-xs text-[var(--text-secondary)] mb-1.5">Target</label>
            <select
              value={selectedTargetId}
              onChange={(e) => setSelectedTargetId(e.target.value)}
              disabled={!selectedAgentId}
              className="w-full px-3 py-2 text-sm rounded bg-[var(--bg-primary)] border border-[var(--border-default)] focus:border-[var(--accent)] outline-none disabled:opacity-50"
            >
              <option value="">Select a target...</option>
              {targets?.map((t) => (
                <option key={t.id} value={t.id}>{t.display_name ?? t.address} ({t.probe_method})</option>
              ))}
            </select>
          </div>
        </div>
      </div>

      {/* Summary Statistics */}
      {summary && selectedTarget && (
        <div className="bg-[var(--bg-surface)] border border-[var(--border-default)] rounded-lg">
          <div className="p-3 border-b border-[var(--border-default)] flex items-center gap-2">
            <BarChart3 className="w-4 h-4 text-[var(--accent)]" />
            <h2 className="text-sm font-semibold">
              Summary: {selectedTarget.display_name ?? selectedTarget.address}
            </h2>
          </div>
          <div className="grid grid-cols-4 gap-4 p-4">
            <StatCard label="Total Sessions" value={summary.totalSessions.toString()} />
            <StatCard
              label="Active Sessions"
              value={summary.activeSessions.toString()}
              accent={summary.activeSessions > 0}
            />
            <StatCard
              label="Total Samples"
              value={summary.totalSamples.toLocaleString()}
            />
            <StatCard
              label="Monitoring Duration"
              value={formatDuration(summary.totalDurationMs)}
            />
            <StatCard
              label="First Session"
              value={new Date(summary.earliest).toLocaleDateString()}
            />
            <StatCard
              label="Latest Activity"
              value={new Date(summary.latest).toLocaleDateString()}
            />
            <StatCard
              label="Probe Method"
              value={selectedTarget.probe_method.toUpperCase()}
            />
            <StatCard
              label="Avg Samples/Session"
              value={summary.totalSessions > 0
                ? Math.round(summary.totalSamples / summary.totalSessions).toLocaleString()
                : '0'}
            />
          </div>
        </div>
      )}

      {/* Sessions List */}
      {selectedTargetId && (
        <div className="bg-[var(--bg-surface)] border border-[var(--border-default)] rounded-lg">
          <div className="p-3 border-b border-[var(--border-default)]">
            <h2 className="text-sm font-semibold">Trace Sessions</h2>
          </div>
          <div className="overflow-auto">
            <table className="w-full text-xs">
              <thead>
                <tr className="text-[var(--text-secondary)] border-b border-[var(--border-default)]">
                  <th className="px-3 py-2 text-left">Started</th>
                  <th className="px-3 py-2 text-left">Ended</th>
                  <th className="px-3 py-2 text-right">Duration</th>
                  <th className="px-3 py-2 text-right">Samples</th>
                  <th className="px-3 py-2 text-left">Status</th>
                  <th className="px-3 py-2 text-right">Actions</th>
                </tr>
              </thead>
              <tbody>
                {sessions?.map((session) => (
                  <tr key={session.id} className="border-b border-[var(--border-default)] hover:bg-[var(--bg-elevated)] transition-colors">
                    <td className="px-3 py-2 font-mono">
                      {new Date(session.started_at).toLocaleString()}
                    </td>
                    <td className="px-3 py-2 font-mono">
                      {session.ended_at ? new Date(session.ended_at).toLocaleString() : '-'}
                    </td>
                    <td className="px-3 py-2 text-right font-mono text-[var(--text-secondary)]">
                      {formatDuration(
                        (session.ended_at ? new Date(session.ended_at).getTime() : Date.now()) -
                        new Date(session.started_at).getTime()
                      )}
                    </td>
                    <td className="px-3 py-2 text-right font-mono">
                      {session.sample_count.toLocaleString()}
                    </td>
                    <td className="px-3 py-2">
                      <span
                        className={`px-1.5 py-0.5 rounded text-[10px] ${
                          session.ended_at
                            ? 'bg-[var(--text-secondary)]/20 text-[var(--text-secondary)]'
                            : 'bg-[var(--success)]/20 text-[var(--success)]'
                        }`}
                      >
                        {session.ended_at ? 'Ended' : 'Active'}
                      </span>
                    </td>
                    <td className="px-3 py-2 text-right">
                      <button
                        onClick={() => handleExportCSV(session.id)}
                        className="inline-flex items-center gap-1 px-2 py-1 text-[10px] rounded bg-[var(--accent)]/10 text-[var(--accent)] hover:bg-[var(--accent)]/20 transition-colors"
                      >
                        <Download className="w-3 h-3" />
                        CSV
                      </button>
                    </td>
                  </tr>
                ))}
                {sessions?.length === 0 && (
                  <tr>
                    <td colSpan={6} className="px-3 py-6 text-center text-[var(--text-secondary)]">
                      No sessions found for this target
                    </td>
                  </tr>
                )}
                {!sessions && (
                  <tr>
                    <td colSpan={6} className="px-3 py-6 text-center text-[var(--text-secondary)]">
                      Loading sessions...
                    </td>
                  </tr>
                )}
              </tbody>
            </table>
          </div>
        </div>
      )}

      {!selectedTargetId && (
        <div className="bg-[var(--bg-surface)] border border-[var(--border-default)] rounded-lg p-8 text-center">
          <p className="text-[var(--text-secondary)] text-sm">
            Select an agent and target above to view trace sessions and export data.
          </p>
        </div>
      )}
    </div>
  );
}

function StatCard({ label, value, accent }: { label: string; value: string; accent?: boolean }) {
  return (
    <div className="bg-[var(--bg-primary)] rounded-lg px-3 py-2.5">
      <div className="text-[10px] text-[var(--text-secondary)] mb-0.5">{label}</div>
      <div className={`text-sm font-semibold font-mono ${accent ? 'text-[var(--success)]' : ''}`}>
        {value}
      </div>
    </div>
  );
}

function formatDuration(ms: number): string {
  if (ms < 0) return '-';
  const seconds = Math.floor(ms / 1000);
  if (seconds < 60) return `${seconds}s`;
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return `${minutes}m ${seconds % 60}s`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours}h ${minutes % 60}m`;
  const days = Math.floor(hours / 24);
  return `${days}d ${hours % 24}h`;
}
