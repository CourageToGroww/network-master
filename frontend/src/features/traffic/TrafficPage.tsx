import { useEffect, useState } from 'react';
import { ArrowDownToLine, ArrowUpFromLine, Network, Server } from 'lucide-react';
import { useAgents } from '../../api/queries/useAgents';
import { useTrafficStore } from '../../stores/trafficStore';
import { useWS } from '../../ws/WebSocketProvider';
import { ProcessTable } from './ProcessTable';
import type { ProcessTrafficSummary } from '../../types';

function formatBytes(bytesPerSec: number): string {
  if (bytesPerSec < 1024) return `${bytesPerSec.toFixed(0)} B/s`;
  if (bytesPerSec < 1024 * 1024) return `${(bytesPerSec / 1024).toFixed(1)} KB/s`;
  return `${(bytesPerSec / 1024 / 1024).toFixed(2)} MB/s`;
}

const EMPTY_PROCESSES: ProcessTrafficSummary[] = [];

export function TrafficPage() {
  const { data: agents, isLoading } = useAgents();
  const [selectedAgentId, setSelectedAgentId] = useState<string | null>(null);
  const { subscribeTraffic, unsubscribeTraffic } = useWS();
  const agentState = useTrafficStore((s) =>
    selectedAgentId ? s.agents.get(selectedAgentId) ?? null : null
  );
  const processes = agentState?.latest?.processes ?? EMPTY_PROCESSES;
  const latest = agentState?.latest ?? null;

  // Auto-select first online agent
  useEffect(() => {
    if (!selectedAgentId && agents?.length) {
      const online = agents.find((a) => a.is_online);
      setSelectedAgentId(online?.id ?? agents[0].id);
    }
  }, [agents, selectedAgentId]);

  // Subscribe to traffic for selected agent (survives WebSocket reconnects)
  useEffect(() => {
    if (!selectedAgentId) return;
    subscribeTraffic([selectedAgentId]);
    return () => {
      unsubscribeTraffic([selectedAgentId]);
    };
  }, [selectedAgentId, subscribeTraffic, unsubscribeTraffic]);

  const totalIn = processes.reduce((sum, p) => sum + p.bytes_in_per_sec, 0);
  const totalOut = processes.reduce((sum, p) => sum + p.bytes_out_per_sec, 0);
  const totalConns = processes.reduce((sum, p) => sum + p.active_connections, 0);

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <h1 className="text-lg font-semibold">Network Traffic</h1>
        <select
          className="px-3 py-1.5 text-sm rounded border border-[var(--border-default)] bg-[var(--bg-surface)] text-[var(--text-primary)]"
          value={selectedAgentId ?? ''}
          onChange={(e) => setSelectedAgentId(e.target.value || null)}
        >
          <option value="">Select agent...</option>
          {agents?.map((agent) => (
            <option key={agent.id} value={agent.id}>
              {agent.name || agent.hostname || agent.id.slice(0, 8)}
              {agent.is_online ? '' : ' (offline)'}
            </option>
          ))}
        </select>
      </div>

      {/* Summary Cards */}
      <div className="grid grid-cols-4 gap-4">
        <div className="p-4 rounded-lg bg-[var(--bg-surface)] border border-[var(--border-default)]">
          <div className="text-xs text-[var(--text-secondary)] flex items-center gap-1">
            <ArrowDownToLine className="w-3 h-3" /> Download
          </div>
          <div className="text-2xl font-bold mt-1 text-green-400">{formatBytes(totalIn)}</div>
        </div>
        <div className="p-4 rounded-lg bg-[var(--bg-surface)] border border-[var(--border-default)]">
          <div className="text-xs text-[var(--text-secondary)] flex items-center gap-1">
            <ArrowUpFromLine className="w-3 h-3" /> Upload
          </div>
          <div className="text-2xl font-bold mt-1 text-blue-400">{formatBytes(totalOut)}</div>
        </div>
        <div className="p-4 rounded-lg bg-[var(--bg-surface)] border border-[var(--border-default)]">
          <div className="text-xs text-[var(--text-secondary)] flex items-center gap-1">
            <Network className="w-3 h-3" /> Connections
          </div>
          <div className="text-2xl font-bold mt-1">{totalConns}</div>
        </div>
        <div className="p-4 rounded-lg bg-[var(--bg-surface)] border border-[var(--border-default)]">
          <div className="text-xs text-[var(--text-secondary)] flex items-center gap-1">
            <Server className="w-3 h-3" /> Processes
          </div>
          <div className="text-2xl font-bold mt-1">{processes.length}</div>
        </div>
      </div>

      {/* Process Table */}
      {isLoading ? (
        <div className="text-sm text-[var(--text-secondary)] p-8 text-center">Loading agents...</div>
      ) : !selectedAgentId ? (
        <div className="text-sm text-[var(--text-secondary)] p-8 text-center">Select an agent to view traffic</div>
      ) : !latest ? (
        <div className="text-sm text-[var(--text-secondary)] p-8 text-center">
          Waiting for traffic data from agent...
        </div>
      ) : (
        <ProcessTable processes={processes} />
      )}
    </div>
  );
}
