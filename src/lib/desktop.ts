import { invoke } from '@tauri-apps/api/core';

export interface AppHealth {
  appName: string;
  appVersion: string;
  platform: 'macos';
  databaseSchemaVersion: number;
}

type Invoke = <T>(command: string) => Promise<T>;

export function getAppHealth(invokeFn: Invoke = invoke): Promise<AppHealth> {
  return invokeFn<AppHealth>('get_app_health');
}
