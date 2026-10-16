import type { ReactNode } from 'react';
import { Sidebar } from './Sidebar';
import { Toolbar } from './Toolbar';
import './AppShell.css';
export type AppView = 'overview' | 'workspaces';

export function AppShell({
  children,
  currentView,
  onNavigate,
}: {
  children: ReactNode;
  currentView: AppView;
  onNavigate: (view: AppView) => void;
}) {
  return (
    <div className="app-shell">
      <a className="skip-link" href="#main-content">
        Skip to content
      </a>
      <Sidebar currentView={currentView} onNavigate={onNavigate} />
      <div className="primary-pane">
        <Toolbar
          title={currentView === 'overview' ? 'Overview' : 'Workspaces'}
        />
        <main
          id="main-content"
          className="content-pane"
          tabIndex={0}
          aria-label={`${currentView === 'overview' ? 'Overview' : 'Workspaces'} content`}
        >
          {children}
        </main>
      </div>
    </div>
  );
}
