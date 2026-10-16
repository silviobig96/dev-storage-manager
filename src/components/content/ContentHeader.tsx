import './content.css';

export function ContentHeader({
  title,
  description,
}: {
  title: string;
  description?: string;
}) {
  return (
    <div className="content-header">
      <h1>{title}</h1>
      {description && <p>{description}</p>}
    </div>
  );
}
