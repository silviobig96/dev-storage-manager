import type { HealthState } from '../App';
import { ContentHeader } from '../components/content/ContentHeader';
import { Section } from '../components/content/Section';
import { DataList } from '../components/content/DataList';
import { ListRow } from '../components/content/ListRow';
import { EmptyState } from '../components/content/EmptyState';
import { AppIcon } from '../components/ui/AppIcon';
import { StatusIndicator } from '../components/ui/StatusIndicator';
import './OverviewView.css';
import type { ReactNode, Ref } from 'react';
import type { WorkspaceScanController } from '../features/scans/useWorkspaceScan';
import { ScanSummary } from '../features/scans/ScanSection';

export function OverviewView({
  healthState,
  headingRef,
  setup,
  scan,
}: {
  healthState: HealthState;
  headingRef?: Ref<HTMLHeadingElement>;
  setup?: ReactNode;
  scan?: WorkspaceScanController;
}) {
  return (
    <div className="overview-view">
      <ContentHeader
        title="Overview"
        headingRef={headingRef}
        description="Developer storage on this Mac."
      />
      {setup}
      <Section title="Storage overview">
        {scan?.snapshot ? (
          <ScanSummary scan={scan} />
        ) : (
          <EmptyState
            title="No storage analysis yet"
            description="Start an explicit scan from Workspaces to measure configured folders."
          />
        )}
      </Section>
      <Section title="Safety posture">
        <DataList>
          <ListRow
            icon={<AppIcon name="lock" />}
            label="Local-first"
            description="Your information stays on this Mac. No telemetry or network access."
          />
          <ListRow
            icon={<AppIcon name="check" />}
            label="No automatic deletion"
            description="Cleanup will always require explicit selection and confirmation."
          />
        </DataList>
      </Section>
      <div
        className="foundation-health"
        role="status"
        aria-live="polite"
        aria-atomic="true"
      >
        {healthState.status === 'loading' && (
          <StatusIndicator>Connecting to desktop foundation…</StatusIndicator>
        )}
        {healthState.status === 'connected' && (
          <>
            <StatusIndicator>Foundation connected</StatusIndicator>
            <span className="numeric">
              Schema {healthState.health.databaseSchemaVersion}
            </span>
          </>
        )}
        {healthState.status === 'unavailable' && (
          <StatusIndicator tone="error">
            Desktop core is unavailable. Restart the app and try again.
          </StatusIndicator>
        )}
      </div>
    </div>
  );
}
