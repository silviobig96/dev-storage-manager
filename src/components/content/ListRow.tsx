import type { ReactNode } from 'react';
import './content.css';

export function ListRow({
  label,
  description,
  trailing,
  icon,
}: {
  label: ReactNode;
  description?: string;
  trailing?: ReactNode;
  icon?: ReactNode;
}) {
  return (
    <li className="list-row">
      {icon && <span className="list-row-icon">{icon}</span>}
      <div className="list-row-body">
        <div className="list-row-label">{label}</div>
        {description && <p>{description}</p>}
      </div>
      {trailing && <div className="list-row-trailing">{trailing}</div>}
    </li>
  );
}
