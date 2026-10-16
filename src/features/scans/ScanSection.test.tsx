import { act, fireEvent, render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { ScanSection, WorkspaceMeasurement } from './ScanSection';
import { scanController, scanMetrics, scanSnapshot } from './scanTestFixtures';
import { ScanCommandError, type ScanDiagnosticCode } from '../../lib/scans';

describe('explicit workspace scan presentation', () => {
  it('requires enabled roots and never starts on render or scope changes', () => {
    const scan = scanController();
    const { rerender } = render(
      <ScanSection scan={scan} enabledIds={[]} configurationBusy={false} />,
    );
    expect(
      screen.getByText(
        'No enabled workspace folders. Add or enable a workspace to scan.',
      ),
    ).toBeVisible();
    expect(screen.getByRole('button', { name: 'Quick scan' })).toBeDisabled();
    rerender(
      <ScanSection scan={scan} enabledIds={['1']} configurationBusy={false} />,
    );
    expect(screen.getByRole('button', { name: 'Deep scan' })).toBeEnabled();
    expect(screen.getByText('Deep scan also has limits.')).toBeVisible();
    expect(scan.start).not.toHaveBeenCalled();
  });
  it('blocks oversized scope and an open picker without selecting a subset', () => {
    const scan = scanController();
    const { rerender } = render(
      <ScanSection
        scan={scan}
        enabledIds={Array.from({ length: 33 }, (_, i) => String(i + 1))}
        configurationBusy={false}
      />,
    );
    expect(
      screen.getByText(
        'A scan supports up to 32 enabled workspaces. Disable some workspaces to scan.',
      ),
    ).toBeVisible();
    fireEvent.click(screen.getByRole('button', { name: 'Quick scan' }));
    expect(scan.start).not.toHaveBeenCalled();
    rerender(<ScanSection scan={scan} enabledIds={['1']} configurationBusy />);
    expect(screen.getByRole('button', { name: 'Quick scan' })).toBeDisabled();
    expect(screen.getByRole('button', { name: 'Deep scan' })).toBeDisabled();
  });
  it.each(['quick', 'deep'] as const)(
    'captures %s intent once under double activation',
    async (mode) => {
      let finish!: () => void;
      const completion = new Promise<void>((resolve) => {
        finish = resolve;
      });
      const scan = scanController({
        start: vi.fn(() => completion),
      });
      const ids = ['1', '2'];
      render(
        <ScanSection scan={scan} enabledIds={ids} configurationBusy={false} />,
      );
      const button = screen.getByRole('button', {
        name: mode === 'quick' ? 'Quick scan' : 'Deep scan',
      });
      fireEvent.click(button);
      fireEvent.click(button);
      ids.push('3');
      expect(scan.start).toHaveBeenCalledExactlyOnceWith(['1', '2'], mode);
      await act(async () => {
        finish();
        await completion;
      });
    },
  );
  it('reconnects without replay and only explicitly replays the original pending intent', async () => {
    const scan = scanController({
      status: 'unavailable',
      error: new ScanCommandError('application_unavailable', 'transport'),
      pendingStart: { workspaceIds: ['9'], mode: 'deep' },
    });
    render(
      <ScanSection scan={scan} enabledIds={['1']} configurationBusy={false} />,
    );
    expect(
      screen.getByText(
        'Scan status is unavailable. Reconnect to check progress.',
      ),
    ).toBeVisible();
    expect(screen.getByRole('button', { name: 'Quick scan' })).toBeDisabled();
    await userEvent.click(screen.getByRole('button', { name: 'Reconnect' }));
    expect(scan.reconnect).toHaveBeenCalledOnce();
    expect(scan.start).not.toHaveBeenCalled();
    expect(
      screen.getByText(/retries the original Deep scan request/i),
    ).toBeVisible();
    await userEvent.click(screen.getByRole('button', { name: 'Deep scan' }));
    expect(scan.start).toHaveBeenCalledExactlyOnceWith(['9'], 'deep');
  });
  it.each(['validating', 'running', 'cancelling', 'stopping'] as const)(
    'keeps %s live until actual worker stop',
    async (state) => {
      const scan = scanController({
        snapshot: scanSnapshot({ state, workerActive: true, finishedAt: null }),
      });
      render(
        <ScanSection
          scan={scan}
          enabledIds={['1']}
          configurationBusy={false}
        />,
      );
      expect(screen.getByRole('button', { name: 'Quick scan' })).toBeDisabled();
      expect(screen.getByRole('button', { name: 'Cancel scan' })).toBeEnabled();
      if (state === 'stopping')
        expect(
          screen.getByText(
            'Waiting for a filesystem operation to finish. No new scan can start yet.',
          ),
        ).toBeVisible();
      await userEvent.click(
        screen.getByRole('button', { name: 'Cancel scan' }),
      );
      expect(scan.cancel).toHaveBeenCalledOnce();
    },
  );
  it('does not announce counters or move focus as progress changes', () => {
    const snapshot = scanSnapshot({ state: 'running', workerActive: true });
    const scan = scanController({ snapshot });
    const { rerender } = render(
      <ScanSection scan={scan} enabledIds={['1']} configurationBusy={false} />,
    );
    const cancel = screen.getByRole('button', { name: 'Cancel scan' });
    cancel.focus();
    const status = screen.getByRole('status');
    expect(status).toHaveTextContent('Scanning…');
    expect(status).not.toHaveTextContent('1,234');
    rerender(
      <ScanSection
        scan={{
          ...scan,
          snapshot: { ...snapshot, entryAttempts: '9007199254740993' },
        }}
        enabledIds={['1']}
        configurationBusy={false}
      />,
    );
    expect(
      screen.getByText('Traversal checks attempted: 9,007,199,254,740,993'),
    ).toBeVisible();
    expect(status).toHaveTextContent('Scanning…');
    expect(cancel).toHaveFocus();
    expect(screen.queryByRole('progressbar')).not.toBeInTheDocument();
  });
  it('qualifies exact logical and allocated measurements and terminal scope', () => {
    render(
      <ScanSection
        scan={scanController({ snapshot: scanSnapshot() })}
        enabledIds={['1']}
        configurationBusy={false}
      />,
    );
    expect(
      screen.getByText('Scan completed within the declared scope.'),
    ).toBeVisible();
    expect(screen.getByText('1 GiB')).toBeVisible();
    expect(screen.getByText('1,073,741,824 bytes')).toBeVisible();
    expect(
      screen.getByText('Observed file sizes; not reclaimable space.'),
    ).toBeVisible();
    expect(
      screen.queryByRole('button', { name: /clean|delete|reclaim/i }),
    ).not.toBeInTheDocument();
  });
  it.each([
    [
      'depth_limit',
      'Partial measurement — traversal stopped at a scan limit.',
      'Depth limit',
    ],
    [
      'entry_limit',
      'Partial measurement — traversal stopped at a scan limit.',
      'Entry limit',
    ],
    [
      'identity_limit',
      'Partial measurement — traversal stopped at a scan limit.',
      'Identity limit',
    ],
    [
      'descriptor_limit',
      'Partial measurement — traversal stopped at a scan limit.',
      'Descriptor limit',
    ],
    [
      'diagnostic_limit',
      'Partial measurement — traversal stopped at a scan limit.',
      'Diagnostic limit',
    ],
    [
      'result_limit',
      'Partial measurement — traversal stopped at a scan limit.',
      'Payload limit',
    ],
    [
      'time_budget',
      'Partial measurement — traversal stopped at a scan limit.',
      'Time budget',
    ],
    [
      'root_denied',
      'Partial measurement — some folders could not be checked or changed during the scan.',
      '',
    ],
    [
      'root_changed',
      'Partial measurement — some folders could not be checked or changed during the scan.',
      '',
    ],
    [
      'allocation_unknown',
      'File sizes measured; allocated space is only partially known.',
      '',
    ],
    ['cancelled', 'Scan cancelled. Measurements shown are incomplete.', ''],
  ])('explains %s from actual results', (code, copy, reason) => {
    const snapshot = scanSnapshot({
      state: code === 'cancelled' ? 'cancelled' : 'partial',
      coverage: 'partial',
      metrics: scanMetrics({
        observedAllocatedBytes: null,
        allocatedCoverage: 'unavailable',
        allocationUnknownFiles: '1',
      }),
      diagnostics: [
        {
          workspaceId: '1',
          code: code as ScanDiagnosticCode,
          severity: 'warning',
          count: '1',
          message: 'Fixed diagnostic.',
        },
      ],
    });
    render(
      <ScanSection
        scan={scanController({ snapshot })}
        enabledIds={['1']}
        configurationBusy={false}
      />,
    );
    expect(screen.getByText(copy)).toBeVisible();
    if (reason) expect(screen.getByText(new RegExp(reason))).toBeVisible();
    if (code === 'allocation_unknown')
      expect(
        screen.queryByText(/some folders could not/),
      ).not.toBeInTheDocument();
    expect(screen.getAllByText('Not available').length).toBeGreaterThan(0);
  });
  it('names only actual policy exclusions without inventing a limit', () => {
    const snapshot = scanSnapshot({
      diagnostics: [
        {
          workspaceId: '1',
          code: 'symlink_excluded',
          severity: 'info',
          count: '2',
          message: 'Excluded.',
        },
      ],
    });
    render(
      <ScanSection
        scan={scanController({ snapshot })}
        enabledIds={['1']}
        configurationBusy={false}
      />,
    );
    expect(
      screen.getByText(
        'Declared exclusions: symbolic links (2) are not included.',
      ),
    ).toBeVisible();
    expect(
      screen.queryByText(/mount crossings|scan limit/),
    ).not.toBeInTheDocument();
  });
  it('distinguishes unmeasured roots, measured zero, and zero attributed elsewhere', () => {
    const snapshot = scanSnapshot();
    const root = snapshot.workspaces[0]!;
    const { rerender } = render(
      <WorkspaceMeasurement
        result={{ ...root, metrics: null, coverage: 'unavailable' }}
        workspaces={snapshot.workspaces}
      />,
    );
    expect(screen.getAllByText(/Not available/).length).toBeGreaterThan(0);
    expect(screen.queryByText('0 B')).not.toBeInTheDocument();
    rerender(
      <WorkspaceMeasurement
        result={{
          ...root,
          metrics: scanMetrics({
            observedLogicalBytes: '0',
            observedAllocatedBytes: '0',
          }),
        }}
        workspaces={snapshot.workspaces}
      />,
    );
    expect(screen.getAllByText('0 B')).toHaveLength(2);
    rerender(
      <WorkspaceMeasurement
        result={{
          ...root,
          metrics: scanMetrics({
            observedLogicalBytes: '0',
            observedAllocatedBytes: '0',
          }),
          coveredByWorkspaceIds: ['2'],
        }}
        workspaces={[
          root,
          { ...root, workspaceId: '2', displayName: 'Parent' },
        ]}
      />,
    );
    expect(
      screen.getByText(/Accounted for in another workspace: Parent/),
    ).toBeVisible();
  });
  it.each(['1073741824', '0'])(
    'does not describe mixed contribution %s with allocated bytes as wholly accounted elsewhere',
    (logical) => {
      const root = scanSnapshot().workspaces[0]!;
      render(
        <WorkspaceMeasurement
          result={{
            ...root,
            metrics: scanMetrics({ observedLogicalBytes: logical }),
            coveredByWorkspaceIds: ['2'],
          }}
          workspaces={[
            root,
            { ...root, workspaceId: '2', displayName: 'Parent' },
          ]}
        />,
      );
      expect(
        screen.getByText(/Some entries are accounted for in: Parent/),
      ).toBeVisible();
      expect(
        screen.queryByText(/Accounted for in another workspace:/),
      ).not.toBeInTheDocument();
      expect(screen.getByText('4 KiB')).toBeVisible();
    },
  );
  it.each(['reconciling', 'unavailable'] as const)(
    'keeps retained completion explicitly stale during %s',
    (status) => {
      render(
        <ScanSection
          scan={scanController({ status, snapshot: scanSnapshot() })}
          enabledIds={['1']}
          configurationBusy={false}
        />,
      );
      expect(screen.getByRole('button', { name: 'Quick scan' })).toBeDisabled();
      expect(screen.getByText(/Last known measurements/)).toBeVisible();
      expect(
        within(screen.getByRole('status')).queryByText(
          'Scan completed within the declared scope.',
        ),
      ).not.toBeInTheDocument();
    },
  );
});
