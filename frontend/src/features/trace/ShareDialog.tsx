import { useState } from 'react';
import { useMutation } from '@tanstack/react-query';
import { Copy, Link, Trash2, X } from 'lucide-react';
import { useShareTokens } from '../../api/queries/useShares';
import { api, queryClient } from '../../api/client';
import type { ShareToken } from '../../types';

interface ShareDialogProps {
  targetId: string;
  onClose: () => void;
}

export function ShareDialog({ targetId, onClose }: ShareDialogProps) {
  const { data: shares } = useShareTokens(targetId);
  const [label, setLabel] = useState('');
  const [expiresInHours, setExpiresInHours] = useState<string>('');
  const [copied, setCopied] = useState<string | null>(null);

  const createShare = useMutation({
    mutationFn: () =>
      api.post<ShareToken>(`/targets/${targetId}/shares`, {
        target_id: targetId,
        label: label || null,
        is_readonly: true,
        expires_in_hours: expiresInHours ? Number(expiresInHours) : null,
      }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['share-tokens', targetId] });
      setLabel('');
      setExpiresInHours('');
    },
  });

  const deleteShare = useMutation({
    mutationFn: (id: string) => api.delete(`/shares/${id}`),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['share-tokens', targetId] });
    },
  });

  const copyLink = (token: string) => {
    const url = `${window.location.origin}/share/${token}`;
    navigator.clipboard.writeText(url);
    setCopied(token);
    setTimeout(() => setCopied(null), 2000);
  };

  return (
    <div className="fixed inset-0 bg-black/50 z-50 flex items-center justify-center" onClick={onClose}>
      <div
        className="bg-[var(--bg-surface)] border border-[var(--border-default)] rounded-lg w-[500px] max-h-[70vh] overflow-auto shadow-xl"
        onClick={(e) => e.stopPropagation()}
      >
        {/* Header */}
        <div className="p-3 border-b border-[var(--border-default)] flex items-center justify-between">
          <div className="flex items-center gap-2">
            <Link className="w-4 h-4 text-[var(--accent)]" />
            <h2 className="text-sm font-semibold">Share Target</h2>
          </div>
          <button onClick={onClose} className="text-[var(--text-secondary)] hover:text-[var(--text-primary)]">
            <X className="w-4 h-4" />
          </button>
        </div>

        {/* Create new share */}
        <div className="p-3 border-b border-[var(--border-default)]">
          <div className="flex gap-2">
            <input
              type="text"
              value={label}
              onChange={(e) => setLabel(e.target.value)}
              placeholder="Share label (optional)"
              className="flex-1 px-2 py-1.5 text-xs rounded bg-[var(--bg-primary)] border border-[var(--border-default)] focus:border-[var(--accent)] outline-none"
            />
            <select
              value={expiresInHours}
              onChange={(e) => setExpiresInHours(e.target.value)}
              className="px-2 py-1.5 text-xs rounded bg-[var(--bg-primary)] border border-[var(--border-default)] focus:border-[var(--accent)] outline-none"
            >
              <option value="">Never expires</option>
              <option value="1">1 hour</option>
              <option value="24">24 hours</option>
              <option value="168">7 days</option>
              <option value="720">30 days</option>
            </select>
            <button
              onClick={() => createShare.mutate()}
              disabled={createShare.isPending}
              className="px-3 py-1.5 text-xs rounded bg-[var(--accent)] text-white hover:opacity-90 transition-opacity disabled:opacity-50"
            >
              {createShare.isPending ? 'Creating...' : 'Create Link'}
            </button>
          </div>
        </div>

        {/* Existing shares */}
        <div className="p-3 space-y-2">
          {shares?.map((share) => (
            <div
              key={share.id}
              className="flex items-center gap-2 p-2 rounded bg-[var(--bg-primary)] border border-[var(--border-default)]"
            >
              <div className="flex-1 min-w-0">
                <div className="text-xs font-medium truncate">
                  {share.label || 'Unnamed share'}
                </div>
                <div className="text-[10px] text-[var(--text-secondary)] font-mono truncate">
                  {window.location.origin}/share/{share.token}
                </div>
                <div className="text-[10px] text-[var(--text-secondary)]">
                  {share.expires_at
                    ? `Expires ${new Date(share.expires_at).toLocaleDateString()}`
                    : 'Never expires'}
                </div>
              </div>
              <button
                onClick={() => copyLink(share.token)}
                className={`px-2 py-1 text-[10px] rounded transition-colors ${
                  copied === share.token
                    ? 'bg-[var(--success)]/20 text-[var(--success)]'
                    : 'bg-[var(--accent)]/10 text-[var(--accent)] hover:bg-[var(--accent)]/20'
                }`}
              >
                <Copy className="w-3 h-3" />
              </button>
              <button
                onClick={() => deleteShare.mutate(share.id)}
                className="text-[var(--text-secondary)] hover:text-[var(--danger)] transition-colors"
              >
                <Trash2 className="w-3 h-3" />
              </button>
            </div>
          ))}
          {shares?.length === 0 && (
            <p className="text-xs text-[var(--text-secondary)] text-center py-4">
              No share links yet. Create one above.
            </p>
          )}
        </div>
      </div>
    </div>
  );
}
