import type { ReactNode } from 'react';
import { AppIcon, type AppIconName } from './AppIcon';
import './controls.css';

type Tone = 'neutral' | 'safe' | 'review' | 'protected' | 'error';
const statusIcons: Record<Tone, AppIconName> = {
  neutral: 'info',
  safe: 'check',
  review: 'warning',
  protected: 'lock',
  error: 'warning',
};

export function StatusIndicator({
  tone = 'neutral',
  children,
}: {
  tone?: Tone;
  children: ReactNode;
}) {
  return (
    <span className={`status-indicator status-indicator--${tone}`}>
      <AppIcon name={statusIcons[tone]} />
      <span>{children}</span>
    </span>
  );
}
