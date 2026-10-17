import type { ReactNode } from "react";

export function PageHeader({
  title,
  description,
  accent,
  actions,
}: {
  title: string;
  description?: string;
  /** A small mark under the title, like the overview's time-of-day rail. */
  accent?: ReactNode;
  actions?: ReactNode;
}) {
  return (
    <header className="flex flex-wrap items-end justify-between gap-4">
      <div className="min-w-0">
        <h1 className="font-display font-wide text-3xl font-bold tracking-tight">
          {title}
        </h1>
        {accent}
        {description ? (
          <p className="mt-1.5 max-w-prose text-sm text-muted-foreground">
            {description}
          </p>
        ) : null}
      </div>
      {actions ? (
        <div className="flex flex-wrap items-center gap-2">{actions}</div>
      ) : null}
    </header>
  );
}
