import type { ReactNode } from 'react';
import { Sidebar } from './Sidebar';
import { Toolbar } from './Toolbar';
import './AppShell.css';

export function AppShell({
  children,
  currentView,
}: {
  children: ReactNode;
  currentView: 'overview';
}) {
  return (
    <div className="app-shell">
      <a className="skip-link" href="#main-content">
        Skip to content
      </a>
      <Sidebar />
      <div className="primary-pane">
        <Toolbar
          title={currentView === 'overview' ? 'Overview' : currentView}
        />
        <main
          id="main-content"
          className="content-pane"
          tabIndex={0}
          aria-label="Overview content"
        >
          {children}
        </main>
      </div>
    </div>
  );
}
