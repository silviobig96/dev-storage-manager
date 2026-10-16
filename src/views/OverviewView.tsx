import type { HealthState } from '../App';
import { ContentHeader } from '../components/content/ContentHeader';
import { Section } from '../components/content/Section';
import { DataList } from '../components/content/DataList';
import { ListRow } from '../components/content/ListRow';
import { EmptyState } from '../components/content/EmptyState';
import { AppIcon } from '../components/ui/AppIcon';
import { StatusIndicator } from '../components/ui/StatusIndicator';
import './OverviewView.css';

export function OverviewView({ healthState }: { healthState: HealthState }) {
  return (
    <div className="overview-view">
      <ContentHeader
        title="Overview"
        description="Developer storage on this Mac."
      />
      <Section title="Storage overview">
        <EmptyState
          title="No storage analysis yet"
          description="Storage analysis is not available in this foundation release. No files have been scanned."
        />
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
