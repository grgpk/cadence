import { useQuery } from "@tanstack/react-query";
import { createFileRoute } from "@tanstack/react-router";

import { Skeleton } from "../../components/ui/skeleton";
import { hostSettingsQuery } from "../../domains/dashboard/host-api";
import { CallFlow } from "../../domains/integrations/call-flow";
import {
  COMING_SOON_CALENDARS,
  COMING_SOON_CONFERENCING,
} from "../../domains/integrations/catalog";
import { ComingSoon } from "../../domains/integrations/coming-soon";
import { callPlace } from "../../domains/integrations/conferencing";
import { ConferencingForm } from "../../domains/integrations/conferencing-form";
import { GoogleCalendarConnection } from "../../domains/integrations/google-calendar-connection";
import { ErrorState } from "../../domains/shared/error-state";
import { PageHeader } from "../../domains/shared/page-header";
import { PanelCard } from "../../domains/shared/panel-card";
import { bookingPageQuery } from "../../domains/widget/cadence-api";
import { APP_NAME } from "../../lib/config";

export const Route = createFileRoute("/dashboard/integrations")({
  head: () => ({ meta: [{ title: `Integrations | ${APP_NAME}` }] }),
  component: IntegrationsPage,
});

function IntegrationsPage() {
  const settings = useQuery(hostSettingsQuery);
  const slug = settings.data?.public_slug;
  const page = useQuery({ ...bookingPageQuery(slug ?? ""), enabled: Boolean(slug) });

  return (
    <>
      <PageHeader
        title="Integrations"
        description="Connect your calendar and choose where your calls happen."
      />
      {settings.isPending || (slug && page.isPending) ? (
        <div className="grid max-w-3xl gap-6">
          <Skeleton className="h-28 w-full rounded-xl" />
          <Skeleton className="h-40 w-full rounded-xl" />
          <Skeleton className="h-72 w-full rounded-xl" />
        </div>
      ) : settings.isError ? (
        <ErrorState
          title="Your integrations did not load"
          error={settings.error}
          onRetry={() => void settings.refetch()}
        />
      ) : (
        <div className="grid max-w-3xl gap-6">
          <CallFlow
            calendarConnected={page.data?.calendar_connected ?? false}
            place={callPlace({
              app: settings.data.conferencing_app,
              calendarConnected: page.data?.calendar_connected ?? false,
              hasLink: Boolean(settings.data.meeting_link),
            })}
          />

          <PanelCard
            title="Calendar"
            description="Busy times on your calendar are kept off your booking page, and each new call is added to it."
            footer={<ComingSoon apps={COMING_SOON_CALENDARS} />}
          >
            <GoogleCalendarConnection slug={settings.data.public_slug} />
          </PanelCard>

          <PanelCard
            title="Meeting app"
            description="Where new calls happen. People who book get the link in their confirmation. Calls already booked keep theirs."
            footer={<ComingSoon apps={COMING_SOON_CONFERENCING} />}
          >
            <ConferencingForm
              saved={settings.data}
              calendarConnected={page.data?.calendar_connected ?? false}
            />
          </PanelCard>
        </div>
      )}
    </>
  );
}
