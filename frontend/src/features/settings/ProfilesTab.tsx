import { useState } from 'react';
import { useMutation } from '@tanstack/react-query';
import { Plus, Trash2, Layers } from 'lucide-react';
import { useTraceProfiles } from '../../api/queries/useProfiles';
import { api, queryClient } from '../../api/client';
import type { TraceProfile } from '../../types';

const PROBE_METHODS = [
  { value: 'icmp', label: 'ICMP' },
  { value: 'tcp', label: 'TCP' },
  { value: 'udp', label: 'UDP' },
];

interface ProfileFormData {
  name: string;
  description: string;
  probe_method: string;
  probe_port: string;
  packet_size: string;
  interval_ms: string;
  max_hops: string;
}

const defaultForm: ProfileFormData = {
  name: '',
  description: '',
  probe_method: 'icmp',
  probe_port: '',
  packet_size: '64',
  interval_ms: '2500',
  max_hops: '30',
};

export function ProfilesTab() {
  const { data: profiles } = useTraceProfiles();
  const [showForm, setShowForm] = useState(false);
  const [form, setForm] = useState<ProfileFormData>(defaultForm);

  const createProfile = useMutation({
    mutationFn: (data: ProfileFormData) =>
      api.post<TraceProfile>('/trace-profiles', {
        name: data.name,
        description: data.description || null,
        probe_method: data.probe_method,
        probe_port: data.probe_port ? Number(data.probe_port) : null,
        packet_size: Number(data.packet_size),
        interval_ms: Number(data.interval_ms),
        max_hops: Number(data.max_hops),
      }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['trace-profiles'] });
      setShowForm(false);
      setForm(defaultForm);
    },
  });

  const deleteProfile = useMutation({
    mutationFn: (id: string) => api.delete(`/trace-profiles/${id}`),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['trace-profiles'] });
    },
  });

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!form.name.trim()) return;
    createProfile.mutate(form);
  };

  return (
    <div className="space-y-4">
      <div className="bg-[var(--bg-surface)] border border-[var(--border-default)] rounded-lg">
        <div className="p-3 border-b border-[var(--border-default)] flex items-center justify-between">
          <div className="flex items-center gap-2">
            <Layers className="w-4 h-4 text-[var(--accent)]" />
            <h2 className="text-sm font-semibold">Trace Profiles</h2>
            <span className="text-xs text-[var(--text-secondary)]">({profiles?.length ?? 0})</span>
          </div>
          <button
            onClick={() => setShowForm(!showForm)}
            className="flex items-center gap-1 px-2.5 py-1 text-xs rounded bg-[var(--accent)] text-white hover:opacity-90 transition-opacity"
          >
            <Plus className="w-3 h-3" />
            New Profile
          </button>
        </div>

        {showForm && (
          <form onSubmit={handleSubmit} className="p-4 border-b border-[var(--border-default)] bg-[var(--bg-elevated)]">
            <div className="grid grid-cols-3 gap-3">
              <div>
                <label className="block text-[10px] text-[var(--text-secondary)] mb-1">Name</label>
                <input
                  type="text"
                  value={form.name}
                  onChange={(e) => setForm({ ...form, name: e.target.value })}
                  placeholder="My Custom Profile"
                  className="w-full px-2 py-1.5 text-xs rounded bg-[var(--bg-primary)] border border-[var(--border-default)] focus:border-[var(--accent)] outline-none"
                />
              </div>
              <div className="col-span-2">
                <label className="block text-[10px] text-[var(--text-secondary)] mb-1">Description</label>
                <input
                  type="text"
                  value={form.description}
                  onChange={(e) => setForm({ ...form, description: e.target.value })}
                  placeholder="Optional description..."
                  className="w-full px-2 py-1.5 text-xs rounded bg-[var(--bg-primary)] border border-[var(--border-default)] focus:border-[var(--accent)] outline-none"
                />
              </div>
              <div>
                <label className="block text-[10px] text-[var(--text-secondary)] mb-1">Probe Method</label>
                <select
                  value={form.probe_method}
                  onChange={(e) => setForm({ ...form, probe_method: e.target.value })}
                  className="w-full px-2 py-1.5 text-xs rounded bg-[var(--bg-primary)] border border-[var(--border-default)] focus:border-[var(--accent)] outline-none"
                >
                  {PROBE_METHODS.map((m) => (
                    <option key={m.value} value={m.value}>{m.label}</option>
                  ))}
                </select>
              </div>
              <div>
                <label className="block text-[10px] text-[var(--text-secondary)] mb-1">Port (TCP/UDP only)</label>
                <input
                  type="number"
                  value={form.probe_port}
                  onChange={(e) => setForm({ ...form, probe_port: e.target.value })}
                  placeholder="80"
                  disabled={form.probe_method === 'icmp'}
                  className="w-full px-2 py-1.5 text-xs rounded bg-[var(--bg-primary)] border border-[var(--border-default)] focus:border-[var(--accent)] outline-none font-mono disabled:opacity-50"
                />
              </div>
              <div>
                <label className="block text-[10px] text-[var(--text-secondary)] mb-1">Packet Size (bytes)</label>
                <input
                  type="number"
                  value={form.packet_size}
                  onChange={(e) => setForm({ ...form, packet_size: e.target.value })}
                  className="w-full px-2 py-1.5 text-xs rounded bg-[var(--bg-primary)] border border-[var(--border-default)] focus:border-[var(--accent)] outline-none font-mono"
                />
              </div>
              <div>
                <label className="block text-[10px] text-[var(--text-secondary)] mb-1">Interval (ms)</label>
                <input
                  type="number"
                  value={form.interval_ms}
                  onChange={(e) => setForm({ ...form, interval_ms: e.target.value })}
                  className="w-full px-2 py-1.5 text-xs rounded bg-[var(--bg-primary)] border border-[var(--border-default)] focus:border-[var(--accent)] outline-none font-mono"
                />
              </div>
              <div>
                <label className="block text-[10px] text-[var(--text-secondary)] mb-1">Max Hops</label>
                <input
                  type="number"
                  value={form.max_hops}
                  onChange={(e) => setForm({ ...form, max_hops: e.target.value })}
                  className="w-full px-2 py-1.5 text-xs rounded bg-[var(--bg-primary)] border border-[var(--border-default)] focus:border-[var(--accent)] outline-none font-mono"
                />
              </div>
            </div>
            <div className="flex justify-end gap-2 mt-3">
              <button
                type="button"
                onClick={() => { setShowForm(false); setForm(defaultForm); }}
                className="px-3 py-1.5 text-xs rounded text-[var(--text-secondary)] hover:bg-[var(--bg-surface)] transition-colors"
              >
                Cancel
              </button>
              <button
                type="submit"
                disabled={createProfile.isPending || !form.name.trim()}
                className="px-3 py-1.5 text-xs rounded bg-[var(--accent)] text-white hover:opacity-90 transition-opacity disabled:opacity-50"
              >
                {createProfile.isPending ? 'Creating...' : 'Create Profile'}
              </button>
            </div>
          </form>
        )}

        <div className="overflow-auto">
          <table className="w-full text-xs">
            <thead>
              <tr className="text-[var(--text-secondary)] border-b border-[var(--border-default)]">
                <th className="px-3 py-2 text-left">Name</th>
                <th className="px-3 py-2 text-left">Description</th>
                <th className="px-3 py-2 text-left">Method</th>
                <th className="px-3 py-2 text-right">Port</th>
                <th className="px-3 py-2 text-right">Interval</th>
                <th className="px-3 py-2 text-right">Max Hops</th>
                <th className="px-3 py-2 text-right">Pkt Size</th>
                <th className="px-3 py-2 text-right w-12"></th>
              </tr>
            </thead>
            <tbody>
              {profiles?.map((p) => (
                <tr key={p.id} className="border-b border-[var(--border-default)] hover:bg-[var(--bg-elevated)] transition-colors">
                  <td className="px-3 py-2 font-medium">{p.name}</td>
                  <td className="px-3 py-2 text-[var(--text-secondary)] truncate max-w-[200px]">{p.description ?? '-'}</td>
                  <td className="px-3 py-2 font-mono uppercase">{p.probe_method}</td>
                  <td className="px-3 py-2 text-right font-mono">{p.probe_port ?? '-'}</td>
                  <td className="px-3 py-2 text-right font-mono">{p.interval_ms}ms</td>
                  <td className="px-3 py-2 text-right font-mono">{p.max_hops}</td>
                  <td className="px-3 py-2 text-right font-mono">{p.packet_size}B</td>
                  <td className="px-3 py-2 text-right">
                    <button
                      onClick={() => deleteProfile.mutate(p.id)}
                      className="text-[var(--text-secondary)] hover:text-[var(--danger)] transition-colors"
                    >
                      <Trash2 className="w-3.5 h-3.5" />
                    </button>
                  </td>
                </tr>
              ))}
              {profiles?.length === 0 && (
                <tr>
                  <td colSpan={8} className="px-3 py-6 text-center text-[var(--text-secondary)]">
                    No profiles configured. Click "New Profile" to create one.
                  </td>
                </tr>
              )}
            </tbody>
          </table>
        </div>
      </div>
    </div>
  );
}
