import { useParams } from 'react-router-dom';
import { useQuery } from '@tanstack/react-query';
import { Eye, Lock } from 'lucide-react';
import { api } from '../../api/client';
import type { Target } from '../../types';

interface SharedTargetInfo {
  target: Target;
  share_label: string | null;
  is_readonly: boolean;
}

export function SharedTracePage() {
  const { token } = useParams<{ token: string }>();

  const { data, isLoading, error } = useQuery({
    queryKey: ['shared-target', token],
    queryFn: () => api.get<SharedTargetInfo>(`/share/${token}`),
    enabled: !!token,
  });

  if (isLoading) {
    return (
      <div className="min-h-screen bg-[var(--bg-primary)] flex items-center justify-center">
        <p className="text-[var(--text-secondary)]">Loading shared trace...</p>
      </div>
    );
  }

  if (error || !data) {
    return (
      <div className="min-h-screen bg-[var(--bg-primary)] flex items-center justify-center">
        <div className="bg-[var(--bg-surface)] border border-[var(--border-default)] rounded-lg p-8 text-center max-w-md">
          <Lock className="w-8 h-8 text-[var(--text-secondary)] mx-auto mb-3" />
          <h1 className="text-lg font-semibold mb-2">Share Link Invalid</h1>
          <p className="text-sm text-[var(--text-secondary)]">
            This share link is invalid, expired, or has been revoked.
          </p>
        </div>
      </div>
    );
  }

  const { target, share_label } = data;

  return (
    <div className="min-h-screen bg-[var(--bg-primary)] p-6">
      <div className="max-w-4xl mx-auto space-y-6">
        {/* Header */}
        <div className="bg-[var(--bg-surface)] border border-[var(--border-default)] rounded-lg p-4">
          <div className="flex items-center gap-3">
            <Eye className="w-5 h-5 text-[var(--accent)]" />
            <div>
              <h1 className="text-lg font-semibold">
                {share_label ?? 'Shared Trace'}
              </h1>
              <p className="text-sm text-[var(--text-secondary)]">
                Read-only view of trace target: {target.display_name ?? target.address}
              </p>
            </div>
            <div className="ml-auto">
              <span className="px-2 py-1 text-[10px] rounded bg-[var(--accent)]/10 text-[var(--accent)]">
                Read-Only
              </span>
            </div>
          </div>
        </div>

        {/* Target Info */}
        <div className="bg-[var(--bg-surface)] border border-[var(--border-default)] rounded-lg p-4">
          <h2 className="text-sm font-semibold mb-3">Target Details</h2>
          <div className="grid grid-cols-3 gap-4">
            <InfoItem label="Address" value={target.address} />
            <InfoItem label="Display Name" value={target.display_name ?? '-'} />
            <InfoItem label="Probe Method" value={target.probe_method.toUpperCase()} />
            <InfoItem label="Interval" value={`${target.interval_ms}ms`} />
            <InfoItem label="Max Hops" value={target.max_hops.toString()} />
            <InfoItem label="Packet Size" value={`${target.packet_size} bytes`} />
            <InfoItem
              label="Status"
              value={target.is_active ? 'Active' : 'Inactive'}
              accent={target.is_active}
            />
            {target.probe_port && (
              <InfoItem label="Port" value={target.probe_port.toString()} />
            )}
          </div>
        </div>

        <div className="bg-[var(--bg-surface)] border border-[var(--border-default)] rounded-lg p-8 text-center">
          <p className="text-sm text-[var(--text-secondary)]">
            Live trace data will appear here when the target is actively being traced.
          </p>
        </div>
      </div>
    </div>
  );
}

function InfoItem({ label, value, accent }: { label: string; value: string; accent?: boolean }) {
  return (
    <div>
      <div className="text-[10px] text-[var(--text-secondary)]">{label}</div>
      <div className={`text-sm font-mono ${accent ? 'text-[var(--success)]' : ''}`}>{value}</div>
    </div>
  );
}
