import { Link, createFileRoute } from "@tanstack/react-router";
import { ChevronRight } from "lucide-react";

import { Card } from "../../components/ui/card";
import { GoogleCalendarCard } from "../../domains/admin/components/GoogleCalendarCard";
import { ADMIN_PAGES } from "../../domains/admin/routes";
import { PageHeader } from "../../domains/shared/PageHeader";

export const Route = createFileRoute("/admin/")({
  component: AdminOverviewPage,
});

function AdminOverviewPage() {
  return (
    <>
      <PageHeader
        title="Admin"
        description="Leads, calls, access and application health."
      />
      <Card className="gap-0 divide-y overflow-hidden py-0">
        {Object.values(ADMIN_PAGES).map(({ title, description, to, icon: Icon }) => (
          <Link
            key={to}
            to={to}
            className="flex items-center gap-4 px-5 py-4 outline-none hover:bg-muted focus-visible:bg-muted"
          >
            <span className="flex size-9 shrink-0 items-center justify-center rounded-lg border bg-muted text-muted-foreground">
              <Icon className="size-4" />
            </span>
            <span className="min-w-0 flex-1">
              <span className="block font-medium">{title}</span>
              <span className="block text-sm text-muted-foreground">{description}</span>
            </span>
            <ChevronRight className="size-4 shrink-0 text-muted-foreground" />
          </Link>
        ))}
      </Card>
      <GoogleCalendarCard />
    </>
  );
}
