import { useNavigate } from 'react-router-dom';
import { Server } from 'lucide-react';
import { useAgents } from '../../api/queries/useAgents';
import { useTargets } from '../../api/queries/useTargets';
import { useDashboardSummary } from '../../api/queries/useAlerts';
import { useAgentStore } from '../../stores/agentStore';

export function DashboardPage() {
  const navigate = useNavigate();
  const { data: agents, isLoading: agentsLoading } = useAgents();
  const { data: summary } = useDashboardSummary();
  const { selectedAgentId, selectAgent } = useAgentStore();
  const { data: targets } = useTargets(selectedAgentId ?? '');

  return (
    <div className="space-y-6">
      {/* Summary Cards */}
      <div className="grid grid-cols-4 gap-4">
        {[
          { label: 'Online Agents', value: summary?.online_agents ?? 0, total: summary?.total_agents },
          { label: 'Active Targets', value: summary?.active_targets ?? 0, total: summary?.total_targets },
          { label: 'Active Alerts', value: summary?.active_alerts ?? 0 },
          { label: 'Samples (24h)', value: summary?.total_samples_24h?.toLocaleString() ?? '0' },
        ].map(({ label, value, total }) => (
          <div
            key={label}
            className="p-4 rounded-lg bg-[var(--bg-surface)] border border-[var(--border-default)]"
          >
            <div className="text-xs text-[var(--text-secondary)]">{label}</div>
            <div className="text-2xl font-bold mt-1">
              {value}
              {total !== undefined && (
                <span className="text-sm text-[var(--text-secondary)] font-normal">/{total}</span>
              )}
            </div>
          </div>
        ))}
      </div>

      <div className="grid grid-cols-[300px_1fr] gap-4">
        {/* Agent List */}
        <div className="bg-[var(--bg-surface)] border border-[var(--border-default)] rounded-lg">
          <div className="p-3 border-b border-[var(--border-default)] flex items-center justify-between">
            <h2 className="text-sm font-semibold">Agents</h2>
          </div>
          <div className="divide-y divide-[var(--border-default)]">
            {agentsLoading && <div className="p-4 text-sm text-[var(--text-secondary)]">Loading...</div>}
            {agents?.map((agent) => (
              <button
                key={agent.id}
                onClick={() => selectAgent(agent.id)}
                className={`w-full text-left p-3 hover:bg-[var(--bg-elevated)] transition-colors ${
                  selectedAgentId === agent.id ? 'bg-[var(--bg-elevated)]' : ''
                }`}
              >
                <div className="flex items-center gap-2">
                  <div
                    className={`w-2 h-2 rounded-full ${
                      agent.is_online ? 'bg-[var(--success)]' : 'bg-[var(--text-secondary)]'
                    }`}
                  />
                  <Server className="w-3.5 h-3.5 text-[var(--text-secondary)]" />
                  <span className="text-sm font-medium">{agent.name}</span>
                </div>
                <div className="text-xs text-[var(--text-secondary)] mt-0.5 ml-6">
                  {agent.hostname ?? agent.ip_address ?? 'unknown'}
                </div>
              </button>
            ))}
            {agents?.length === 0 && (
              <div className="p-4 text-sm text-[var(--text-secondary)] text-center">
                No agents registered. Use the CLI to register one.
              </div>
            )}
          </div>
        </div>

        {/* Target List */}
        <div className="bg-[var(--bg-surface)] border border-[var(--border-default)] rounded-lg">
          <div className="p-3 border-b border-[var(--border-default)] flex items-center justify-between">
            <h2 className="text-sm font-semibold">
              {selectedAgentId ? 'Targets' : 'Select an agent'}
            </h2>
          </div>
          <div className="divide-y divide-[var(--border-default)]">
            {targets?.map((target) => (
              <button
                key={target.id}
                onClick={() => navigate(`/trace/${target.agent_id}/${target.id}`)}
                className="w-full text-left p-3 hover:bg-[var(--bg-elevated)] transition-colors flex items-center justify-between"
              >
                <div>
                  <div className="text-sm font-medium">
                    {target.display_name ?? target.address}
                  </div>
                  <div className="text-xs text-[var(--text-secondary)]">
                    {target.address} | {target.probe_method.toUpperCase()} | {target.interval_ms}ms
                  </div>
                </div>
                <div
                  className={`text-xs px-2 py-0.5 rounded ${
                    target.is_active
                      ? 'bg-[var(--success)]/20 text-[var(--success)]'
                      : 'bg-[var(--text-secondary)]/20 text-[var(--text-secondary)]'
                  }`}
                >
                  {target.is_active ? 'Active' : 'Paused'}
                </div>
              </button>
            ))}
            {selectedAgentId && targets?.length === 0 && (
              <div className="p-4 text-sm text-[var(--text-secondary)] text-center">
                No targets. Use the CLI to add one.
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
