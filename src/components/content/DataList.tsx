import type { ReactNode } from 'react';
import './content.css';

export function DataList({ children }: { children: ReactNode }) {
  return <ul className="data-list">{children}</ul>;
}
