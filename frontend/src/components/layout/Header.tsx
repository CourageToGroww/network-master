import { Activity } from 'lucide-react';
import { useWS } from '../../ws/WebSocketProvider';

export function Header() {
  const { status } = useWS();

  return (
    <header className="h-12 border-b border-[var(--border-default)] bg-[var(--bg-surface)] flex items-center px-4 justify-between shrink-0">
      <div className="flex items-center gap-2">
        <Activity className="w-5 h-5 text-[var(--accent)]" />
        <span className="font-semibold text-sm">Network Master</span>
      </div>
      <div className="flex items-center gap-3">
        <div className="flex items-center gap-1.5 text-xs text-[var(--text-secondary)]">
          <div
            className={`w-2 h-2 rounded-full ${
              status === 'connected'
                ? 'bg-[var(--success)]'
                : status === 'reconnecting'
                ? 'bg-[var(--warning)] animate-pulse'
                : 'bg-[var(--danger)]'
            }`}
          />
          {status}
        </div>
      </div>
    </header>
  );
}
