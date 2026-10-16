import { Link } from "@tanstack/react-router";
import { Lock } from "lucide-react";

import { Badge } from "../../components/ui/badge";
import { ROUTES } from "../../lib/routes";

/** Says a setting needs a paid plan, next to the controls it locks. */
export function PlanNotice({ children }: { children: string }) {
  return (
    <div className="flex flex-wrap items-center gap-x-3 gap-y-2 rounded-lg border bg-muted/40 px-3 py-2.5 text-sm">
      <Badge variant="outline">
        <Lock />
        Pro
      </Badge>
      <p className="min-w-0 flex-1 basis-64 text-muted-foreground">{children}</p>
      <Link
        to={ROUTES.dashboardBilling}
        className="font-medium underline underline-offset-4"
      >
        See plans
      </Link>
    </div>
  );
}
