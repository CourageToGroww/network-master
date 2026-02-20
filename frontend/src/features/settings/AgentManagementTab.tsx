import { useState } from 'react';
import { useAgents } from '../../api/queries/useAgents';
import { useUpdateInfo, useTriggerUpdate, useTriggerUpdateAll } from '../../api/queries/useUpdate';
import { UpdateUploadDialog } from './UpdateUploadDialog';

export function AgentManagementTab() {
  const { data: agents } = useAgents();
  const { data: updateInfo } = useUpdateInfo();
  const triggerUpdate = useTriggerUpdate();
  const triggerAll = useTriggerUpdateAll();
  const [showUpload, setShowUpload] = useState(false);
  const [updatingAgents, setUpdatingAgents] = useState<Set<string>>(new Set());

  const handleUpdate = (agentId: string) => {
    setUpdatingAgents((prev) => new Set(prev).add(agentId));
    triggerUpdate.mutate(agentId, {
      onSettled: () => {
        // Keep the "updating" state — the WS update_status messages
        // will handle UI feedback. Remove after a timeout.
        setTimeout(() => {
          setUpdatingAgents((prev) => {
            const next = new Set(prev);
            next.delete(agentId);
            return next;
          });
        }, 30_000);
      },
    });
  };

  const hasUpdate = (agentVersion: string | null): boolean => {
    if (!updateInfo || !agentVersion) return false;
    return agentVersion !== updateInfo.version;
  };

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-3">
          {updateInfo && (
            <span className="text-xs text-[var(--text-secondary)]">
              Latest binary: v{updateInfo.version}
            </span>
          )}
        </div>
        <div className="flex items-center gap-2">
          {updateInfo && (
            <button
              onClick={() => triggerAll.mutate()}
              disabled={triggerAll.isPending}
              className="px-3 py-1.5 text-xs rounded border border-[var(--accent)] text-[var(--accent)] hover:bg-[var(--accent)] hover:text-white disabled:opacity-50"
            >
              {triggerAll.isPending ? 'Pushing...' : 'Update All Agents'}
            </button>
          )}
          <button
            onClick={() => setShowUpload(true)}
            className="px-3 py-1.5 text-xs rounded bg-[var(--accent)] text-white hover:opacity-90"
          >
            Upload Binary
          </button>
        </div>
      </div>

      <div className="bg-[var(--bg-surface)] border border-[var(--border-default)] rounded-lg">
        <table className="w-full text-sm">
          <thead>
            <tr className="text-[var(--text-secondary)] text-xs border-b border-[var(--border-default)]">
              <th className="px-3 py-2 text-left">Name</th>
              <th className="px-3 py-2 text-left">Hostname</th>
              <th className="px-3 py-2 text-left">Version</th>
              <th className="px-3 py-2 text-left">Status</th>
              <th className="px-3 py-2 text-left">Last Seen</th>
              <th className="px-3 py-2 text-left">Actions</th>
            </tr>
          </thead>
          <tbody>
            {agents?.map((agent) => {
              const canUpdate = agent.is_online && hasUpdate(agent.version);
              const isUpdating = updatingAgents.has(agent.id);

              return (
                <tr key={agent.id} className="border-b border-[var(--border-default)]">
                  <td className="px-3 py-2">{agent.name}</td>
                  <td className="px-3 py-2 font-mono text-xs">{agent.hostname ?? '-'}</td>
                  <td className="px-3 py-2 font-mono text-xs">
                    <span>{agent.version ?? '-'}</span>
                    {canUpdate && (
                      <span className="ml-2 px-1.5 py-0.5 rounded text-[10px] bg-[#f59e0b]/20 text-[#f59e0b]">
                        Update Available
                      </span>
                    )}
                  </td>
                  <td className="px-3 py-2">
                    <div className="flex items-center gap-1.5">
                      <div
                        className={`w-2 h-2 rounded-full ${agent.is_online ? 'bg-[var(--success)]' : 'bg-[var(--text-secondary)]'}`}
                      />
                      <span className="text-xs">
                        {agent.is_online ? 'Online' : 'Offline'}
                      </span>
                    </div>
                  </td>
                  <td className="px-3 py-2 text-xs text-[var(--text-secondary)]">
                    {agent.last_seen_at
                      ? new Date(agent.last_seen_at).toLocaleString()
                      : 'Never'}
                  </td>
                  <td className="px-3 py-2">
                    {canUpdate && !isUpdating && (
                      <button
                        onClick={() => handleUpdate(agent.id)}
                        className="px-2 py-1 text-xs rounded bg-[var(--accent)] text-white hover:opacity-90"
                      >
                        Update
                      </button>
                    )}
                    {isUpdating && (
                      <span className="text-xs text-[#f59e0b] animate-pulse">
                        Updating...
                      </span>
                    )}
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>

      <UpdateUploadDialog open={showUpload} onClose={() => setShowUpload(false)} />
    </div>
  );
}
