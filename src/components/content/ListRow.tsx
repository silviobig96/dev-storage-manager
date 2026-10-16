import type { ReactNode, Ref } from 'react';
import './content.css';

export function ListRow({
  label,
  description,
  trailing,
  icon,
  className = '',
  rowRef,
  tabIndex,
  id,
}: {
  label: ReactNode;
  description?: string;
  trailing?: ReactNode;
  icon?: ReactNode;
  className?: string;
  rowRef?: Ref<HTMLLIElement>;
  tabIndex?: number;
  id?: string;
}) {
  return (
    <li
      className={`list-row ${className}`}
      ref={rowRef}
      tabIndex={tabIndex}
      id={id}
    >
      {icon && <span className="list-row-icon">{icon}</span>}
      <div className="list-row-body">
        <div className="list-row-label">{label}</div>
        {description && <p>{description}</p>}
      </div>
      {trailing && <div className="list-row-trailing">{trailing}</div>}
    </li>
  );
}
