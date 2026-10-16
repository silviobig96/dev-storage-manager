import type { ReactNode } from 'react';
import './content.css';

export function DataList({ children }: { children: ReactNode }) {
  // Keep list navigation in WebKit when CSS removes the list markers.
  return (
    <ul className="data-list" role="list">
      {children}
    </ul>
  );
}
