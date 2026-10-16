import { useId, type ReactNode } from 'react';
import './content.css';

export function Section({
  title,
  children,
}: {
  title: string;
  children: ReactNode;
}) {
  const titleId = useId();
  return (
    <section className="content-section" aria-labelledby={titleId}>
      <h2 id={titleId}>{title}</h2>
      {children}
    </section>
  );
}
