import { AppIcon } from '../ui/AppIcon';
import './content.css';

export function EmptyState({
  title,
  description,
}: {
  title: string;
  description: string;
}) {
  return (
    <div className="empty-state">
      <AppIcon name="categories" />
      <h3>{title}</h3>
      <p>{description}</p>
    </div>
  );
}
