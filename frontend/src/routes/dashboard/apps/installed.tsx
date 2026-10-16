import { useQuery } from "@tanstack/react-query";
import { createFileRoute, Link } from "@tanstack/react-router";
import { ArrowRight } from "lucide-react";

import { Button } from "../../../components/ui/button";
import { Skeleton } from "../../../components/ui/skeleton";
import { hostSettingsQuery } from "../../../domains/dashboard/host-api";
import { CallFlow } from "../../../domains/integrations/call-flow";
import { APP_CATEGORY, type AppCategory } from "../../../domains/integrations/catalog";
import { callPlace } from "../../../domains/integrations/conferencing";
import { ConferencingForm } from "../../../domains/integrations/conferencing-form";
import { GoogleCalendarConnection } from "../../../domains/integrations/google-calendar-connection";
import { ErrorState } from "../../../domains/shared/error-state";
import { PageHeader } from "../../../domains/shared/page-header";
import { PanelCard } from "../../../domains/shared/panel-card";
import { hostPageQuery } from "../../../domains/widget/cadence-api";
import { APP_NAME } from "../../../lib/config";
import { ROUTES } from "../../../lib/routes";

export const Route = createFileRoute("/dashboard/apps/installed")({
  head: () => ({ meta: [{ title: `Installed apps | ${APP_NAME}` }] }),
  component: InstalledAppsPage,
});

/** The way to the store, filtered to the apps that could go in a panel. */
function MoreApps({ category }: { category: AppCategory }) {
  return (
    <Button variant="link" size="sm" className="h-auto p-0 has-[>svg]:px-0" asChild>
      <Link to={ROUTES.dashboardApps} search={{ category }}>
        More apps in the App store
        <ArrowRight />
      </Link>
    </Button>
  );
}

function InstalledAppsPage() {
  const settings = useQuery(hostSettingsQuery);
  const slug = settings.data?.public_slug;
  const page = useQuery({ ...hostPageQuery(slug ?? ""), enabled: Boolean(slug) });

  return (
    <>
      <PageHeader
        title="Installed apps"
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
          title="Your apps did not load"
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
            footer={<MoreApps category={APP_CATEGORY.calendar} />}
          >
            <GoogleCalendarConnection slug={settings.data.public_slug} />
          </PanelCard>

          <PanelCard
            title="Meeting app"
            description="Where new calls happen. People who book get the link in their confirmation. Calls already booked keep theirs."
            footer={<MoreApps category={APP_CATEGORY.conferencing} />}
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
