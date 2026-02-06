import { NavLink, Outlet } from 'react-router-dom';

const tabs = [
  { to: '/settings/agents', label: 'Agents' },
  { to: '/settings/targets', label: 'Targets' },
  { to: '/settings/profiles', label: 'Profiles' },
  { to: '/settings/notifications', label: 'Notifications' },
];

export function SettingsPage() {
  return (
    <div className="space-y-4">
      <h1 className="text-lg font-semibold">Settings</h1>
      <div className="flex gap-1 border-b border-[var(--border-default)]">
        {tabs.map(({ to, label }) => (
          <NavLink
            key={to}
            to={to}
            className={({ isActive }) =>
              `px-3 py-2 text-sm transition-colors border-b-2 ${
                isActive
                  ? 'border-[var(--accent)] text-[var(--text-primary)]'
                  : 'border-transparent text-[var(--text-secondary)] hover:text-[var(--text-primary)]'
              }`
            }
          >
            {label}
          </NavLink>
        ))}
      </div>
      <Outlet />
    </div>
  );
}
