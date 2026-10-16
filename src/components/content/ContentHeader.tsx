import './content.css';
import type { Ref } from 'react';

export function ContentHeader({
  title,
  description,
  headingRef,
}: {
  title: string;
  description?: string;
  headingRef?: Ref<HTMLHeadingElement>;
}) {
  return (
    <div className="content-header">
      <h1 ref={headingRef} tabIndex={-1}>
        {title}
      </h1>
      {description && <p>{description}</p>}
    </div>
  );
}
