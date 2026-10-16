import type { ReactNode } from 'react';
import { AppIcon, type AppIconName } from './AppIcon';
import './feedback.css';

export type FeedbackTone = 'neutral' | 'info' | 'success' | 'warning' | 'error';
export interface FeedbackNotice {
  tone: FeedbackTone;
  message: string;
}

const icons: Record<FeedbackTone, AppIconName> = {
  neutral: 'info',
  info: 'info',
  success: 'check',
  warning: 'warning',
  error: 'error',
};

export function FeedbackMessage({
  tone,
  title,
  children,
  action,
  variant = 'notice',
  announce = true,
}: {
  tone: FeedbackTone;
  title?: string;
  children?: ReactNode;
  action?: ReactNode;
  variant?: 'notice' | 'inline';
  announce?: boolean;
}) {
  return (
    <div
      className={`feedback-message feedback-message--${tone} feedback-message--${variant}`}
      role={announce ? (tone === 'error' ? 'alert' : 'status') : undefined}
      aria-atomic={announce ? true : undefined}
    >
      <AppIcon name={icons[tone]} />
      <div className="feedback-message__body">
        {title && <strong className="feedback-message__title">{title}</strong>}
        {children && (
          <p className="feedback-message__description">{children}</p>
        )}
      </div>
      {action && <div className="feedback-message__action">{action}</div>}
    </div>
  );
}
