import { useEffect, useLayoutEffect, useRef, useState } from 'react';
import { AppShell, type AppView } from './components/shell/AppShell';
import { OverviewView } from './views/OverviewView';
import { WorkspacesView } from './views/WorkspacesView';
import { WorkspaceSetup } from './features/workspaces/WorkspaceSetup';
import { useWorkspaces } from './features/workspaces/useWorkspaces';
import { Button } from './components/ui/Button';
import { getAppHealth, type AppHealth } from './lib/desktop';

export type HealthState =
  | { status: 'loading' }
  | { status: 'connected'; health: AppHealth }
  | { status: 'unavailable' };

function App() {
  const workspaces = useWorkspaces();
  const [currentView, setCurrentView] = useState<AppView>('overview');
  const headingRef = useRef<HTMLHeadingElement>(null);
  const pendingFocus = useRef<'heading' | { row: string } | null>(null);
  const [announcement, setAnnouncement] = useState('');
  const [healthState, setHealthState] = useState<HealthState>({
    status: 'loading',
  });

  useEffect(() => {
    let isCurrent = true;

    getAppHealth()
      .then((health) => {
        if (isCurrent) {
          setHealthState({ status: 'connected', health });
        }
      })
      .catch(() => {
        if (isCurrent) {
          setHealthState({ status: 'unavailable' });
        }
      });

    return () => {
      isCurrent = false;
    };
  }, []);

  useLayoutEffect(() => {
    const target = pendingFocus.current;
    if (target === 'heading') headingRef.current?.focus();
    else if (target)
      document.getElementById(`workspace-${target.row}`)?.focus();
    pendingFocus.current = null;
  }, [currentView, announcement]);

  function navigate(view: AppView) {
    if (view === currentView) {
      headingRef.current?.focus();
      return;
    }
    pendingFocus.current = 'heading';
    setCurrentView(view);
    setAnnouncement('');
  }

  async function addFromSetup() {
    const invoker = document.activeElement as HTMLElement | null;
    try {
      const result = await workspaces.add();
      if (result.status === 'cancelled') {
        invoker?.focus();
        return;
      }
      pendingFocus.current = {
        row:
          result.status === 'duplicate'
            ? result.existingId
            : result.workspace.id,
      };
      setAnnouncement(
        result.status === 'duplicate'
          ? 'This folder is already configured.'
          : 'Workspace added. No files were scanned.',
      );
      setCurrentView('workspaces');
    } catch {
      invoker?.focus();
    }
  }

  return (
    <AppShell currentView={currentView} onNavigate={navigate}>
      {currentView === 'overview' ? (
        <>
          <OverviewView healthState={healthState} headingRef={headingRef} />
          {workspaces.state.status === 'ready' &&
            workspaces.state.listing.setupState === 'undecided' && (
              <>
                <WorkspaceSetup
                  adding={
                    workspaces.adding || !!workspaces.error?.requiresReload
                  }
                  error={workspaces.error}
                  onAdd={addFromSetup}
                  onSkip={() => workspaces.skip()}
                />
                {workspaces.error?.requiresReload && (
                  <Button
                    onClick={() => {
                      void workspaces.reload().catch(() => undefined);
                    }}
                  >
                    Reload workspaces
                  </Button>
                )}
              </>
            )}
        </>
      ) : (
        <WorkspacesView workspaces={workspaces} headingRef={headingRef} />
      )}
      {announcement && !workspaces.error && (
        <p role="status" aria-live="polite">
          {announcement}
        </p>
      )}
    </AppShell>
  );
}

export default App;
