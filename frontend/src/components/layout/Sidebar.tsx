import { NavLink } from 'react-router-dom';
import { LayoutDashboard, Activity, Bell, Settings, FileText } from 'lucide-react';

const navItems = [
  { to: '/', icon: LayoutDashboard, label: 'Dashboard' },
  { to: '/traffic', icon: Activity, label: 'Traffic' },
  { to: '/alerts', icon: Bell, label: 'Alerts' },
  { to: '/reports', icon: FileText, label: 'Reports' },
  { to: '/settings/agents', icon: Settings, label: 'Settings' },
];

export function Sidebar() {
  return (
    <aside className="w-48 border-r border-[var(--border-default)] bg-[var(--bg-surface)] shrink-0">
      <nav className="p-2 space-y-1">
        {navItems.map(({ to, icon: Icon, label }) => (
          <NavLink
            key={to}
            to={to}
            className={({ isActive }) =>
              `flex items-center gap-2 px-3 py-2 rounded text-sm transition-colors ${
                isActive
                  ? 'bg-[var(--accent)] text-white'
                  : 'text-[var(--text-secondary)] hover:bg-[var(--bg-elevated)] hover:text-[var(--text-primary)]'
              }`
            }
            end={to === '/'}
          >
            <Icon className="w-4 h-4" />
            {label}
          </NavLink>
        ))}
      </nav>
    </aside>
  );
}
