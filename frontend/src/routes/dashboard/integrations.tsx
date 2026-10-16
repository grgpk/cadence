import { useQuery } from "@tanstack/react-query";
import { createFileRoute } from "@tanstack/react-router";
import { CalendarDays, Video } from "lucide-react";
import type { ReactNode } from "react";

import { Skeleton } from "../../components/ui/skeleton";
import { hostSettingsQuery } from "../../domains/dashboard/host-api";
import {
  COMING_SOON_CALENDARS,
  COMING_SOON_CONFERENCING,
} from "../../domains/integrations/catalog";
import { ConferencingForm } from "../../domains/integrations/conferencing-form";
import { GoogleCalendarConnection } from "../../domains/integrations/google-calendar-connection";
import {
  ComingSoonCard,
  IntegrationCard,
} from "../../domains/integrations/integration-card";
import { ErrorState } from "../../domains/shared/error-state";
import { PageHeader } from "../../domains/shared/page-header";
import { bookingPageQuery } from "../../domains/widget/cadence-api";
import { APP_NAME } from "../../lib/config";

export const Route = createFileRoute("/dashboard/integrations")({
  head: () => ({ meta: [{ title: `Integrations | ${APP_NAME}` }] }),
  component: IntegrationsPage,
});

function Group({
  title,
  description,
  children,
}: {
  title: string;
  description: string;
  children: ReactNode;
}) {
  return (
    <section className="grid gap-4">
      <div>
        <h2 className="font-semibold">{title}</h2>
        <p className="mt-0.5 text-sm text-muted-foreground">{description}</p>
      </div>
      {children}
    </section>
  );
}

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
      {settings.isPending ? (
        <div className="grid gap-6">
          <Skeleton className="h-40 w-full rounded-xl" />
          <Skeleton className="h-64 w-full rounded-xl" />
        </div>
      ) : settings.isError ? (
        <ErrorState
          title="Your integrations did not load"
          error={settings.error}
          onRetry={() => void settings.refetch()}
        />
      ) : (
        <div className="grid max-w-4xl gap-10">
          <Group
            title="Calendars"
            description="Busy times on a connected calendar are kept off your booking page, and each call is added to it."
          >
            <div className="grid gap-4 md:grid-cols-2">
              <IntegrationCard
                icon={<CalendarDays className="size-4 text-primary" />}
                name="Google Calendar"
                description="Checks your busy times and adds each call, with a Meet link if you pick Google Meet."
              >
                <GoogleCalendarConnection slug={settings.data.public_slug} />
              </IntegrationCard>
              {COMING_SOON_CALENDARS.map((app) => (
                <ComingSoonCard key={app.name} app={app} />
              ))}
            </div>
          </Group>

          <Group
            title="Conferencing"
            description="Where every call happens. People who book are sent the link to join."
          >
            <IntegrationCard
              icon={<Video className="size-4 text-muted-foreground" />}
              name="Default meeting app"
              description="Used for every new call. Calls already booked keep their link."
            >
              {page.isPending && slug ? (
                <Skeleton className="h-40 w-full" />
              ) : (
                <ConferencingForm
                  saved={settings.data}
                  calendarConnected={page.data?.calendar_connected ?? false}
                />
              )}
            </IntegrationCard>
            <div className="grid gap-4 md:grid-cols-2">
              {COMING_SOON_CONFERENCING.map((app) => (
                <ComingSoonCard key={app.name} app={app} />
              ))}
            </div>
          </Group>
        </div>
      )}
    </>
  );
}
