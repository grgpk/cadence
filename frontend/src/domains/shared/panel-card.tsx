import type { ReactNode } from "react";

import { Card } from "../../components/ui/card";
import { Skeleton } from "../../components/ui/skeleton";

/** The heading bar every dashboard card opens with. */
export function PanelHeader({
  title,
  description,
  action,
}: {
  title: ReactNode;
  description?: string;
  action?: ReactNode;
}) {
  return (
    <div className="flex min-h-13 items-center justify-between gap-4 border-b py-2.5 pr-3 pl-5">
      <div className="min-w-0">
        <h2 className="font-semibold">{title}</h2>
        {description ? (
          <p className="mt-0.5 text-sm text-muted-foreground">{description}</p>
        ) : null}
      </div>
      {action}
    </div>
  );
}

/** A titled group of controls, as on the Settings and Installed apps pages. */
export function PanelCard({
  id,
  title,
  description,
  footer,
  children,
}: {
  /** Other pages link to the panel by this anchor. */
  id?: string;
  title: string;
  description: string;
  footer?: ReactNode;
  children: ReactNode;
}) {
  return (
    // No overflow clipping: a sticky save bar inside must be free to stick.
    <Card id={id} className="scroll-mt-20 gap-0 py-0">
      <PanelHeader title={title} description={description} />
      <div className="grid gap-5 p-5">{children}</div>
      {footer ? (
        <div className="rounded-b-xl border-t bg-muted/40 px-5 py-3">{footer}</div>
      ) : null}
    </Card>
  );
}

/** A `PanelCard` while it loads: the same bar and field rhythm, so nothing jumps. */
export function PanelCardSkeleton({ rows = 2 }: { rows?: number }) {
  return (
    <Card className="gap-0 py-0" role="status" aria-label="Loading">
      <div className="grid min-h-13 content-center gap-2 border-b py-3 pl-5">
        <Skeleton className="h-4 w-32" />
        <Skeleton className="h-3 w-64 max-w-[70%]" />
      </div>
      <div className="grid gap-5 p-5">
        {Array.from({ length: rows }, (_, index) => (
          <div key={index} className="grid gap-2">
            <Skeleton className="h-3.5 w-24" />
            <Skeleton className="h-9 w-full" />
          </div>
        ))}
      </div>
    </Card>
  );
}
