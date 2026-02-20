import { createBrowserRouter, RouterProvider } from 'react-router-dom';
import { AppShell } from './components/layout/AppShell';
import { DashboardPage } from './features/dashboard/DashboardPage';
import { TracePage } from './features/trace/TracePage';
import { AlertsPage } from './features/alerts/AlertsPage';
import { SettingsPage } from './features/settings/SettingsPage';
import { AgentManagementTab } from './features/settings/AgentManagementTab';
import { ProfilesTab } from './features/settings/ProfilesTab';
import { ReportsPage } from './features/reports/ReportsPage';
import { SharedTracePage } from './features/share/SharedTracePage';
import { TrafficPage } from './features/traffic/TrafficPage';

const router = createBrowserRouter([
  {
    path: '/',
    element: <AppShell />,
    children: [
      { index: true, element: <DashboardPage /> },
      { path: 'trace/:agentId/:targetId', element: <TracePage /> },
      { path: 'traffic', element: <TrafficPage /> },
      { path: 'alerts', element: <AlertsPage /> },
      {
        path: 'settings',
        element: <SettingsPage />,
        children: [
          { path: 'agents', element: <AgentManagementTab /> },
          { path: 'targets', element: <div className="text-[var(--text-secondary)]">Target management</div> },
          { path: 'profiles', element: <ProfilesTab /> },
          { path: 'notifications', element: <div className="text-[var(--text-secondary)]">Notification settings</div> },
        ],
      },
      { path: 'reports', element: <ReportsPage /> },
    ],
  },
  { path: '/share/:token', element: <SharedTracePage /> },
]);

export function App() {
  return <RouterProvider router={router} />;
}
