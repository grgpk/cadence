import type { ReactNode } from "react";

import { Card } from "../../components/ui/card";

/** A titled group of controls, as on the Settings and Integrations pages. */
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
    <Card id={id} className="scroll-mt-16 gap-0 overflow-hidden py-0">
      <div className="border-b px-5 py-3.5">
        <h2 className="font-semibold">{title}</h2>
        <p className="mt-0.5 text-sm text-muted-foreground">{description}</p>
      </div>
      <div className="grid gap-5 p-5">{children}</div>
      {footer ? <div className="border-t bg-muted/40 px-5 py-3">{footer}</div> : null}
    </Card>
  );
}
