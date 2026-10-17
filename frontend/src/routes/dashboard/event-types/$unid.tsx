import { useQuery } from "@tanstack/react-query";
import { createFileRoute, Link } from "@tanstack/react-router";
import { ArrowLeft } from "lucide-react";

import { Button } from "../../../components/ui/button";
import { Skeleton } from "../../../components/ui/skeleton";
import { hostSettingsQuery } from "../../../domains/dashboard/host-api";
import { EventTypeForm } from "../../../domains/event-types/event-type-form";
import { eventTypesQuery } from "../../../domains/event-types/event-types-api";
import { EmptyState } from "../../../domains/shared/empty-state";
import { ErrorState } from "../../../domains/shared/error-state";
import { PageHeader } from "../../../domains/shared/page-header";
import { APP_NAME } from "../../../lib/config";
import { ROUTES } from "../../../lib/routes";
import { CalendarRange } from "lucide-react";

export const Route = createFileRoute("/dashboard/event-types/$unid")({
  head: () => ({ meta: [{ title: `Event type | ${APP_NAME}` }] }),
  component: EventTypePage,
});

function EventTypePage() {
  const { unid } = Route.useParams();
  const { user } = Route.useRouteContext();
  const types = useQuery(eventTypesQuery);
  const settings = useQuery(hostSettingsQuery);
  const kind = types.data?.find((candidate) => candidate.unid === unid);
  const failed = types.isError ? types : settings.isError ? settings : null;

  return (
    <>
      <div>
        <Button variant="ghost" size="sm" className="-ml-2" asChild>
          <Link to={ROUTES.dashboardEventTypes}>
            <ArrowLeft />
            Event types
          </Link>
        </Button>
      </div>
      <PageHeader
        title={kind?.title ?? "Event type"}
        description="Its own link, length, rules and questions. Your hours and calendar are shared by every call."
      />
      {failed ? (
        <ErrorState
          title="This event type did not load"
          error={failed.error}
          onRetry={() => void failed.refetch()}
        />
      ) : types.data && settings.data ? (
        kind ? (
          <EventTypeForm
            key={kind.unid}
            saved={kind}
            hostName={user?.full_name ?? "You"}
            accent={settings.data.accent_color}
            hostSlug={settings.data.public_slug}
            isLast={types.data.length <= 1}
          />
        ) : (
          <EmptyState
            icon={CalendarRange}
            title="This event type is gone"
            description="It may have been deleted. Pick another one from Event types."
          />
        )
      ) : (
        <Skeleton className="h-96 w-full max-w-2xl rounded-xl" />
      )}
    </>
  );
}
