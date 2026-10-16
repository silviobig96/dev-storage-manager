import { AppIcon, type AppIconName } from '../ui/AppIcon';

export function SidebarItem({
  label,
  icon,
  current = false,
}: {
  label: string;
  icon: AppIconName;
  current?: boolean;
}) {
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
