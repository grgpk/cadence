import type { ReactNode } from "react";

import { Badge } from "../../components/ui/badge";
import { Card } from "../../components/ui/card";
import type { ComingSoonApp } from "./catalog";

/** One app: its mark, name and what it does, with its controls below. */
export function IntegrationCard({
  icon,
  name,
  description,
  badge,
  children,
}: {
  icon: ReactNode;
  name: string;
  description: string;
  badge?: ReactNode;
  children?: ReactNode;
}) {
  return (
    <Card className="gap-4 p-5">
      <div className="flex items-start gap-3">
        <span className="grid size-9 shrink-0 place-items-center rounded-md border bg-muted">
          {icon}
        </span>
        <div className="min-w-0 flex-1">
          <div className="flex flex-wrap items-center gap-2">
            <h3 className="font-semibold">{name}</h3>
            {badge}
          </div>
          <p className="mt-0.5 text-sm text-muted-foreground">{description}</p>
        </div>
      </div>
      {children}
    </Card>
  );
}

export function ComingSoonCard({ app }: { app: ComingSoonApp }) {
  const Icon = app.icon;
  return (
    <IntegrationCard
      icon={<Icon className="size-4 text-muted-foreground" />}
      name={app.name}
      description={app.description}
      badge={<Badge variant="secondary">Coming soon</Badge>}
    />
  );
}
