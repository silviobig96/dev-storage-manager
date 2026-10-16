import { SidebarItem } from './SidebarItem';
import './Sidebar.css';
import type { AppView } from './AppShell';

export function Sidebar({
  currentView,
  onNavigate,
}: {
  currentView: AppView;
  onNavigate: (view: AppView) => void;
}) {
  return (
    <aside className="sidebar">
      <div className="sidebar-identity">dev-storage-manager</div>
      <nav className="sidebar-navigation" aria-label="Main">
        <ul className="sidebar-list">
          <SidebarItem
            label="Overview"
            icon="overview"
            current={currentView === 'overview'}
            onActivate={() => onNavigate('overview')}
          />
          <SidebarItem
            label="Workspaces"
            icon="workspaces"
            current={currentView === 'workspaces'}
            onActivate={() => onNavigate('workspaces')}
          />
        </ul>
        <div className="sidebar-future">
          <p className="sidebar-group-label">Unavailable in this release</p>
          <ul className="sidebar-list">
            <SidebarItem label="Projects" icon="projects" />
            <SidebarItem label="Categories" icon="categories" />
            <SidebarItem label="History" icon="history" />
          </ul>
        </div>
        <ul className="sidebar-list sidebar-utilities">
          <SidebarItem label="Settings" icon="settings" />
        </ul>
      </nav>
      <p className="sidebar-release">Foundation release</p>
    </aside>
  );
}
