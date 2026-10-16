import type { ReactNode } from 'react';

export type AppIconName =
  | 'overview'
  | 'workspaces'
  | 'projects'
  | 'categories'
  | 'history'
  | 'settings'
  | 'sidebar'
  | 'info'
  | 'check'
  | 'warning'
  | 'lock';

const symbols: Record<AppIconName, ReactNode> = {
  overview: (
    <>
      <rect x="2" y="2" width="5" height="6" rx="1" />
      <rect x="10" y="2" width="6" height="3" rx="1" />
      <rect x="2" y="11" width="5" height="5" rx="1" />
      <rect x="10" y="8" width="6" height="8" rx="1" />
    </>
  ),
  workspaces: <path d="M2 5V3.5h5l2 2H16V14.5H2V5Z" />,
  projects: (
    <>
      <path d="m6 5-4 4 4 4m6-8 4 4-4 4m-2-10-2 12" />
    </>
  ),
  categories: (
    <>
      <rect x="2" y="2" width="5" height="5" rx="1" />
      <rect x="11" y="2" width="5" height="5" rx="1" />
      <rect x="2" y="11" width="5" height="5" rx="1" />
      <rect x="11" y="11" width="5" height="5" rx="1" />
    </>
  ),
  history: (
    <>
      <path d="M2 7a7 7 0 1 1 1 7M2 3v4h4" />
      <path d="M9 5v4l3 2" />
    </>
  ),
  settings: (
    <>
      <path d="m7 2-.5 2-2 1-2-.5-1 3 1.5 1.5V11L2 12.5l1.5 2.5 2-.5 2 1L8 17h3l.5-1.5 2-1 2 .5 1.5-2.5-1.5-1.5V9L17 7.5l-1.5-3-2 .5-2-1L11 2Z" />
      <circle cx="9.5" cy="9.5" r="2.5" />
    </>
  ),
  sidebar: (
    <>
      <rect x="2" y="3" width="14" height="12" rx="1.5" />
      <path d="M7 3v12" />
    </>
  ),
  info: (
    <>
      <circle cx="9" cy="9" r="7" />
      <path d="M9 8v5m0-8v.2" />
    </>
  ),
  check: <path d="m4 9 3.5 3.5L14 5.5" />,
  warning: (
    <>
      <path d="m9 2 7 13H2L9 2Z" />
      <path d="M9 7v3m0 2v.2" />
    </>
  ),
  lock: (
    <>
      <rect x="3.5" y="8" width="11" height="8" rx="1.5" />
      <path d="M6 8V5a3 3 0 0 1 6 0v3m-3 3v2" />
    </>
  ),
};

export function AppIcon({ name }: { name: AppIconName }) {
  return (
    <svg
      width="18"
      height="18"
      viewBox="0 0 18 18"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.4"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
      focusable="false"
    >
      {symbols[name]}
    </svg>
  );
}
