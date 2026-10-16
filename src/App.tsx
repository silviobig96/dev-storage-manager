import { useEffect, useState } from 'react';
import { AppShell } from './components/shell/AppShell';
import { OverviewView } from './views/OverviewView';
import { getAppHealth, type AppHealth } from './lib/desktop';

export type HealthState =
  | { status: 'loading' }
  | { status: 'connected'; health: AppHealth }
  | { status: 'unavailable' };

function App() {
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

  return (
    <AppShell currentView="overview">
      <OverviewView healthState={healthState} />
    </AppShell>
  );
}

export default App;
