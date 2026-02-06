import { useWS } from '../../ws/WebSocketProvider';
import { useDashboardSummary } from '../../api/queries/useAlerts';

export function StatusBar() {
  const { status } = useWS();
  const { data: summary } = useDashboardSummary();

  return (
    <footer className="h-6 border-t border-[var(--border-default)] bg-[var(--bg-surface)] flex items-center px-4 text-[10px] text-[var(--text-secondary)] gap-4 shrink-0">
      <span>WS: {status}</span>
      {summary && (
        <>
          <span>Agents: {summary.online_agents}/{summary.total_agents}</span>
          <span>Targets: {summary.active_targets}</span>
          <span>Alerts: {summary.active_alerts}</span>
        </>
      )}
    </footer>
  );
}
