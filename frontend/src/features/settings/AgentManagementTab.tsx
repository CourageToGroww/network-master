import { useAgents } from '../../api/queries/useAgents';

export function AgentManagementTab() {
  const { data: agents } = useAgents();

  return (
    <div className="bg-[var(--bg-surface)] border border-[var(--border-default)] rounded-lg">
      <table className="w-full text-sm">
        <thead>
          <tr className="text-[var(--text-secondary)] text-xs border-b border-[var(--border-default)]">
            <th className="px-3 py-2 text-left">Name</th>
            <th className="px-3 py-2 text-left">Hostname</th>
            <th className="px-3 py-2 text-left">Status</th>
            <th className="px-3 py-2 text-left">Last Seen</th>
            <th className="px-3 py-2 text-left">ID</th>
          </tr>
        </thead>
        <tbody>
          {agents?.map((agent) => (
            <tr key={agent.id} className="border-b border-[var(--border-default)]">
              <td className="px-3 py-2">{agent.name}</td>
              <td className="px-3 py-2 font-mono text-xs">{agent.hostname ?? '-'}</td>
              <td className="px-3 py-2">
                <div className="flex items-center gap-1.5">
                  <div className={`w-2 h-2 rounded-full ${agent.is_online ? 'bg-[var(--success)]' : 'bg-[var(--text-secondary)]'}`} />
                  <span className="text-xs">{agent.is_online ? 'Online' : 'Offline'}</span>
                </div>
              </td>
              <td className="px-3 py-2 text-xs text-[var(--text-secondary)]">
                {agent.last_seen_at ? new Date(agent.last_seen_at).toLocaleString() : 'Never'}
              </td>
              <td className="px-3 py-2 font-mono text-[10px] text-[var(--text-secondary)]">{agent.id}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
