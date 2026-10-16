import './Toolbar.css';

export function Toolbar({ title }: { title: string }) {
  return (
    <header className="toolbar" aria-label="Current view">
      <span>{title}</span>
    </header>
  );
}
