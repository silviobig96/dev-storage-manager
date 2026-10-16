import { AppIcon, type AppIconName } from '../ui/AppIcon';

export function SidebarItem({
  label,
  icon,
  current = false,
  onActivate,
}: {
  label: string;
  icon: AppIconName;
  current?: boolean;
  onActivate?: () => void;
}) {
  if (onActivate)
    return (
      <li>
        <button
          type="button"
          className={`sidebar-item sidebar-item--available${current ? ' sidebar-item--current' : ''}`}
          aria-current={current ? 'page' : undefined}
          onClick={onActivate}
        >
          <AppIcon name={icon} />
          <span>{label}</span>
        </button>
      </li>
    );
  return (
    <li
      className={`sidebar-item${current ? ' sidebar-item--current' : ' sidebar-item--unavailable'}`}
      aria-current={current ? 'page' : undefined}
    >
      <AppIcon name={icon} />
      <span>{label}</span>
      {!current && (
        <span className="visually-hidden"> — Unavailable in this release</span>
      )}
    </li>
  );
}
