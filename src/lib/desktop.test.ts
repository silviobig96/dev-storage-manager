import { describe, expect, it, vi } from 'vitest';
import { getAppHealth } from './desktop';

describe('getAppHealth', () => {
  it('invokes only the read-only health command', async () => {
    const invoke = vi.fn().mockResolvedValue({
      appName: 'dev-storage-manager',
      appVersion: '0.1.0',
      platform: 'macos',
      databaseSchemaVersion: 1,
    });

    await expect(getAppHealth(invoke)).resolves.toMatchObject({
      platform: 'macos',
      databaseSchemaVersion: 1,
    });
    expect(invoke).toHaveBeenCalledWith('get_app_health');
    expect(invoke).toHaveBeenCalledTimes(1);
  });
});
