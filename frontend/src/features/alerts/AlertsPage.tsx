import { useState } from 'react';
import { useMutation } from '@tanstack/react-query';
import { Plus, Trash2, Bell } from 'lucide-react';
import { useAlertRules, useAlertEvents } from '../../api/queries/useAlerts';
import { api, queryClient } from '../../api/client';
import type { AlertRule } from '../../types';

const METRICS = [
  { value: 'avg_rtt', label: 'Avg RTT (ms)' },
  { value: 'max_rtt', label: 'Max RTT (ms)' },
  { value: 'loss_pct', label: 'Packet Loss (%)' },
  { value: 'jitter', label: 'Jitter (ms)' },
];

const COMPARATORS = [
  { value: 'gt', label: '>' },
  { value: 'gte', label: '>=' },
  { value: 'lt', label: '<' },
  { value: 'lte', label: '<=' },
];

interface RuleFormData {
  name: string;
  metric: string;
  comparator: string;
  threshold: string;
  cooldown_seconds: string;
  notify_webhook: string;
}

const defaultForm: RuleFormData = {
  name: '',
  metric: 'avg_rtt',
  comparator: 'gt',
  threshold: '100',
  cooldown_seconds: '300',
  notify_webhook: '',
};

export function AlertsPage() {
  const { data: rules } = useAlertRules();
  const { data: events } = useAlertEvents(50);
  const [showForm, setShowForm] = useState(false);
  const [form, setForm] = useState<RuleFormData>(defaultForm);

  const createRule = useMutation({
    mutationFn: (data: RuleFormData) =>
      api.post<AlertRule>('/alert-rules', {
        name: data.name,
        metric: data.metric,
        comparator: data.comparator,
        threshold: Number(data.threshold),
        window_seconds: 60,
        cooldown_seconds: Number(data.cooldown_seconds),
        notify_webhook: data.notify_webhook || null,
      }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['alert-rules'] });
      setShowForm(false);
      setForm(defaultForm);
    },
  });

  const deleteRule = useMutation({
    mutationFn: (id: string) => api.delete(`/alert-rules/${id}`),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['alert-rules'] });
    },
  });

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!form.name.trim()) return;
    createRule.mutate(form);
  };

  return (
    <div className="space-y-6">
      {/* Alert Rules */}
      <div className="bg-[var(--bg-surface)] border border-[var(--border-default)] rounded-lg">
        <div className="p-3 border-b border-[var(--border-default)] flex items-center justify-between">
          <div className="flex items-center gap-2">
            <Bell className="w-4 h-4 text-[var(--accent)]" />
            <h2 className="text-sm font-semibold">Alert Rules</h2>
            <span className="text-xs text-[var(--text-secondary)]">({rules?.length ?? 0})</span>
          </div>
          <button
            onClick={() => setShowForm(!showForm)}
            className="flex items-center gap-1 px-2.5 py-1 text-xs rounded bg-[var(--accent)] text-white hover:opacity-90 transition-opacity"
          >
            <Plus className="w-3 h-3" />
            New Rule
          </button>
        </div>

        {/* Create Rule Form */}
        {showForm && (
          <form onSubmit={handleSubmit} className="p-4 border-b border-[var(--border-default)] bg-[var(--bg-elevated)]">
            <div className="grid grid-cols-3 gap-3">
              <div>
                <label className="block text-[10px] text-[var(--text-secondary)] mb-1">Rule Name</label>
                <input
                  type="text"
                  value={form.name}
                  onChange={(e) => setForm({ ...form, name: e.target.value })}
                  placeholder="High latency alert"
                  className="w-full px-2 py-1.5 text-xs rounded bg-[var(--bg-primary)] border border-[var(--border-default)] focus:border-[var(--accent)] outline-none"
                />
              </div>
              <div>
                <label className="block text-[10px] text-[var(--text-secondary)] mb-1">Metric</label>
                <select
                  value={form.metric}
                  onChange={(e) => setForm({ ...form, metric: e.target.value })}
                  className="w-full px-2 py-1.5 text-xs rounded bg-[var(--bg-primary)] border border-[var(--border-default)] focus:border-[var(--accent)] outline-none"
                >
                  {METRICS.map((m) => (
                    <option key={m.value} value={m.value}>{m.label}</option>
                  ))}
                </select>
              </div>
              <div>
                <label className="block text-[10px] text-[var(--text-secondary)] mb-1">Condition</label>
                <div className="flex gap-1">
                  <select
                    value={form.comparator}
                    onChange={(e) => setForm({ ...form, comparator: e.target.value })}
                    className="w-16 px-2 py-1.5 text-xs rounded bg-[var(--bg-primary)] border border-[var(--border-default)] focus:border-[var(--accent)] outline-none"
                  >
                    {COMPARATORS.map((c) => (
                      <option key={c.value} value={c.value}>{c.label}</option>
                    ))}
                  </select>
                  <input
                    type="number"
                    step="any"
                    value={form.threshold}
                    onChange={(e) => setForm({ ...form, threshold: e.target.value })}
                    className="flex-1 px-2 py-1.5 text-xs rounded bg-[var(--bg-primary)] border border-[var(--border-default)] focus:border-[var(--accent)] outline-none font-mono"
                  />
                </div>
              </div>
              <div>
                <label className="block text-[10px] text-[var(--text-secondary)] mb-1">Cooldown (s)</label>
                <input
                  type="number"
                  value={form.cooldown_seconds}
                  onChange={(e) => setForm({ ...form, cooldown_seconds: e.target.value })}
                  className="w-full px-2 py-1.5 text-xs rounded bg-[var(--bg-primary)] border border-[var(--border-default)] focus:border-[var(--accent)] outline-none font-mono"
                />
              </div>
              <div className="col-span-2">
                <label className="block text-[10px] text-[var(--text-secondary)] mb-1">Webhook URL (optional)</label>
                <input
                  type="url"
                  value={form.notify_webhook}
                  onChange={(e) => setForm({ ...form, notify_webhook: e.target.value })}
                  placeholder="https://hooks.slack.com/..."
                  className="w-full px-2 py-1.5 text-xs rounded bg-[var(--bg-primary)] border border-[var(--border-default)] focus:border-[var(--accent)] outline-none"
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
                disabled={createRule.isPending || !form.name.trim()}
                className="px-3 py-1.5 text-xs rounded bg-[var(--accent)] text-white hover:opacity-90 transition-opacity disabled:opacity-50"
              >
                {createRule.isPending ? 'Creating...' : 'Create Rule'}
              </button>
            </div>
          </form>
        )}

        <div className="overflow-auto">
          <table className="w-full text-xs">
            <thead>
              <tr className="text-[var(--text-secondary)] border-b border-[var(--border-default)]">
                <th className="px-3 py-2 text-left">Name</th>
                <th className="px-3 py-2 text-left">Metric</th>
                <th className="px-3 py-2 text-left">Condition</th>
                <th className="px-3 py-2 text-left">Cooldown</th>
                <th className="px-3 py-2 text-left">Notification</th>
                <th className="px-3 py-2 text-left">Status</th>
                <th className="px-3 py-2 text-right w-12"></th>
              </tr>
            </thead>
            <tbody>
              {rules?.map((rule) => (
                <tr key={rule.id} className="border-b border-[var(--border-default)] hover:bg-[var(--bg-elevated)] transition-colors">
                  <td className="px-3 py-2 font-medium">{rule.name}</td>
                  <td className="px-3 py-2 font-mono">{rule.metric}</td>
                  <td className="px-3 py-2 font-mono">
                    {rule.comparator} {rule.threshold}
                  </td>
                  <td className="px-3 py-2 font-mono text-[var(--text-secondary)]">
                    {rule.cooldown_seconds}s
                  </td>
                  <td className="px-3 py-2">
                    {rule.notify_webhook ? (
                      <span className="text-[var(--accent)]">Webhook</span>
                    ) : rule.notify_email ? (
                      <span className="text-[var(--accent)]">Email</span>
                    ) : (
                      <span className="text-[var(--text-secondary)]">None</span>
                    )}
                  </td>
                  <td className="px-3 py-2">
                    <span
                      className={`px-1.5 py-0.5 rounded text-[10px] ${
                        rule.is_enabled
                          ? 'bg-[var(--success)]/20 text-[var(--success)]'
                          : 'bg-[var(--text-secondary)]/20 text-[var(--text-secondary)]'
                      }`}
                    >
                      {rule.is_enabled ? 'Enabled' : 'Disabled'}
                    </span>
                  </td>
                  <td className="px-3 py-2 text-right">
                    <button
                      onClick={() => deleteRule.mutate(rule.id)}
                      className="text-[var(--text-secondary)] hover:text-[var(--danger)] transition-colors"
                    >
                      <Trash2 className="w-3.5 h-3.5" />
                    </button>
                  </td>
                </tr>
              ))}
              {rules?.length === 0 && (
                <tr>
                  <td colSpan={7} className="px-3 py-6 text-center text-[var(--text-secondary)]">
                    No alert rules configured. Click "New Rule" to create one.
                  </td>
                </tr>
              )}
            </tbody>
          </table>
        </div>
      </div>

      {/* Alert Events */}
      <div className="bg-[var(--bg-surface)] border border-[var(--border-default)] rounded-lg">
        <div className="p-3 border-b border-[var(--border-default)]">
          <h2 className="text-sm font-semibold">Recent Alert Events</h2>
        </div>
        <div className="overflow-auto">
          <table className="w-full text-xs">
            <thead>
              <tr className="text-[var(--text-secondary)] border-b border-[var(--border-default)]">
                <th className="px-3 py-2 text-left">Time</th>
                <th className="px-3 py-2 text-left">Message</th>
                <th className="px-3 py-2 text-right">Value</th>
                <th className="px-3 py-2 text-right">Threshold</th>
                <th className="px-3 py-2 text-left">Status</th>
              </tr>
            </thead>
            <tbody>
              {events?.map((event) => (
                <tr key={event.id} className="border-b border-[var(--border-default)]">
                  <td className="px-3 py-2 font-mono whitespace-nowrap">
                    {new Date(event.triggered_at).toLocaleString()}
                  </td>
                  <td className="px-3 py-2">{event.message}</td>
                  <td className="px-3 py-2 text-right font-mono">{event.metric_value.toFixed(2)}</td>
                  <td className="px-3 py-2 text-right font-mono">{event.threshold_value.toFixed(2)}</td>
                  <td className="px-3 py-2">
                    <span
                      className={`px-1.5 py-0.5 rounded text-[10px] ${
                        event.resolved_at
                          ? 'bg-[var(--success)]/20 text-[var(--success)]'
                          : 'bg-[var(--danger)]/20 text-[var(--danger)]'
                      }`}
                    >
                      {event.resolved_at ? 'Resolved' : 'Active'}
                    </span>
                  </td>
                </tr>
              ))}
              {events?.length === 0 && (
                <tr>
                  <td colSpan={5} className="px-3 py-6 text-center text-[var(--text-secondary)]">
                    No alert events
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
