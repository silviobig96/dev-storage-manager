import { AppIcon, type AppIconName } from './AppIcon';
import { Button, type ButtonProps } from './Button';

type IconButtonProps = Omit<
  ButtonProps,
  'children' | 'aria-label' | 'title'
> & {
  label: string;
  icon: AppIconName;
};

export function IconButton({
  label,
  icon,
  className = '',
  ...props
}: IconButtonProps) {
  return (
    <Button
      {...props}
      className={`icon-button ${className}`}
      aria-label={label}
      title={label}
    >
      <AppIcon name={icon} />
    </Button>
  );
}
