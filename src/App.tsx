import { useEffect, useState } from 'react';
import './App.css';
import { getAppHealth, type AppHealth } from './lib/desktop';

type HealthState =
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
    <main className="app-shell">
      <div className="page-frame">
        <header className="hero">
          <p className="eyebrow">Storage intelligence, before storage action</p>
          <h1>
            Understand what your development environment is doing to your disk.
          </h1>
          <p className="hero-copy">
            This foundation connects the desktop shell to a read-only core. It
            does not scan or clean your files.
          </p>
        </header>

        <section className="core-status" aria-labelledby="core-status-title">
          <div className="status-heading">
            <span className="status-mark" aria-hidden="true" />
            <h2 id="core-status-title">Desktop core</h2>
          </div>
          <div className="status-message" role="status" aria-live="polite">
            {healthState.status === 'loading' && (
              <p>Connecting to desktop foundation…</p>
            )}
            {healthState.status === 'connected' && (
              <p>
                Foundation connected · schema{' '}
                {healthState.health.databaseSchemaVersion}
              </p>
            )}
            {healthState.status === 'unavailable' && (
              <p>Desktop core is unavailable. Restart the app and try again.</p>
            )}
          </div>
        </section>

        <section className="commitments" aria-labelledby="commitments-title">
          <p className="section-label" id="commitments-title">
            Foundation commitments
          </p>
          <ul className="commitment-grid">
            <li>
              <span className="commitment-index" aria-hidden="true">
                01
              </span>
              <h2>Local-first</h2>
              <p>
                Storage intelligence stays on your Mac, without telemetry or
                network access.
              </p>
            </li>
            <li>
              <span className="commitment-index" aria-hidden="true">
                02
              </span>
              <h2>No automatic deletion</h2>
              <p>
                Cleanup will always require explicit selection and confirmation.
              </p>
            </li>
            <li>
              <span className="commitment-index" aria-hidden="true">
                03
              </span>
              <h2>macOS first</h2>
              <p>Windows support is planned, not included in this phase.</p>
            </li>
          </ul>
        </section>
      </div>
    </main>
  );
}

export default App;
