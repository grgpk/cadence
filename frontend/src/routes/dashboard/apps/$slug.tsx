import { useQuery } from "@tanstack/react-query";
import { createFileRoute, Link, notFound } from "@tanstack/react-router";
import { cn } from "cn";
import { ArrowLeft } from "lucide-react";

import type { HostSettings } from "../../../bindings/HostSettings";

import { Badge } from "../../../components/ui/badge";
import { Button } from "../../../components/ui/button";
import { Skeleton } from "../../../components/ui/skeleton";
import { hostSettingsQuery } from "../../../domains/dashboard/host-api";
import { AppLogo, AppStatus } from "../../../domains/integrations/app-card";
import { CallFlow } from "../../../domains/integrations/call-flow";
import {
  APP_CATEGORY,
  type AppEntry,
  CATEGORY_INFO,
  findApp,
  isInstalled,
} from "../../../domains/integrations/catalog";
import { callPlace } from "../../../domains/integrations/conferencing";
import { ConferencingForm } from "../../../domains/integrations/conferencing-form";
import { GoogleCalendarConnection } from "../../../domains/integrations/google-calendar-connection";
import { ErrorState } from "../../../domains/shared/error-state";
import { PanelCard } from "../../../domains/shared/panel-card";
import { bookingPageQuery } from "../../../domains/widget/cadence-api";
import { APP_NAME } from "../../../lib/config";
import { ROUTES } from "../../../lib/routes";

export const Route = createFileRoute("/dashboard/apps/$slug")({
  loader: ({ params }) => {
    if (!findApp(params.slug)) throw notFound();
  },
  head: ({ params }) => ({
    meta: [{ title: `${findApp(params.slug)?.name ?? "App"} | ${APP_NAME}` }],
  }),
  component: AppDetailsPage,
});

function AppDetailsPage() {
  const app = findApp(Route.useParams().slug);
  const settings = useQuery(hostSettingsQuery);
  const slug = settings.data?.public_slug;
  const page = useQuery({ ...bookingPageQuery(slug ?? ""), enabled: Boolean(slug) });
  if (!app) return null;
  const calendarConnected = page.data?.calendar_connected ?? false;
  const loading = settings.isPending || Boolean(slug && page.isPending);

  return (
    <>
      <Button variant="ghost" size="sm" className="-ml-2 w-fit" asChild>
        <Link to={ROUTES.dashboardApps}>
          <ArrowLeft />
          App store
        </Link>
      </Button>

      <header className="flex items-center gap-4">
        <AppLogo app={app} size="lg" />
        <div className="min-w-0">
          <h1 className="font-display font-wide text-3xl font-bold tracking-tight">
            {app.name}
          </h1>
          <div className="mt-1.5 flex flex-wrap items-center gap-2">
            <Badge variant="secondary">{CATEGORY_INFO[app.category].name}</Badge>
            {settings.data ? (
              <AppStatus
                app={app}
                installed={isInstalled(app.slug, settings.data, calendarConnected)}
              />
            ) : null}
          </div>
        </div>
      </header>

      <div className="grid gap-8 lg:grid-cols-[minmax(0,1fr)_17rem] lg:items-start">
        <div className="grid gap-6">
          {!app.available ? (
            <p className="rounded-xl border border-dashed p-6 text-sm text-muted-foreground">
              {app.name} is on the way. Until then, calls use{" "}
              <Link
                to={ROUTES.dashboardAppsInstalled}
                className="font-medium text-foreground underline underline-offset-4"
              >
                the apps you have
              </Link>
              .
            </p>
          ) : loading ? (
            <>
              <Skeleton className="h-28 w-full rounded-xl" />
              <Skeleton className="h-64 w-full rounded-xl" />
            </>
          ) : settings.isError ? (
            <ErrorState
              title="Your apps did not load"
              error={settings.error}
              onRetry={() => void settings.refetch()}
            />
          ) : (
            <>
              <CallFlow
                calendarConnected={calendarConnected}
                place={callPlace({
                  app: settings.data.conferencing_app,
                  calendarConnected,
                  hasLink: Boolean(settings.data.meeting_link),
                })}
              />
              {app.category === APP_CATEGORY.conferencing ? (
                <PanelCard
                  title="Where new calls happen"
                  description="People who book get this link in their confirmation. Calls already booked keep theirs."
                >
                  <ConferencingForm
                    saved={settings.data}
                    calendarConnected={calendarConnected}
                  />
                </PanelCard>
              ) : (
                <PanelCard
                  title="Your calendar"
                  description="Busy times are kept off your booking page, and each new call is added to it."
                >
                  <GoogleCalendarConnection slug={settings.data.public_slug} />
                </PanelCard>
              )}
            </>
          )}
        </div>

        <About app={app} settings={settings.data} calendarConnected={calendarConnected} />
      </div>
    </>
  );
}

/** What the app is, beside the controls: description and the facts about it. */
function About({
  app,
  settings,
  calendarConnected,
}: {
  app: AppEntry;
  settings: HostSettings | undefined;
  calendarConnected: boolean;
}) {
  const required = app.requires ? findApp(app.requires) : undefined;
  const requiredOn =
    required && settings
      ? isInstalled(required.slug, settings, calendarConnected)
      : false;
  return (
    <aside className="order-first grid gap-6 text-sm lg:order-none lg:border-l lg:pl-6">
      <p className="leading-relaxed">{app.description}</p>
      <dl className="grid grid-cols-[auto_minmax(0,1fr)] gap-x-4 gap-y-2.5">
        <dt className="text-muted-foreground">Price</dt>
        <dd>Free</dd>
        <dt className="text-muted-foreground">Category</dt>
        <dd>{CATEGORY_INFO[app.category].name}</dd>
        {required ? (
          <>
            <dt className="text-muted-foreground">Needs</dt>
            <dd className="grid gap-0.5">
              <Link
                to={ROUTES.dashboardApp}
                params={{ slug: required.slug }}
                className="font-medium underline-offset-4 hover:underline"
              >
                {required.name}
              </Link>
              <span
                className={cn(
                  "flex items-center gap-1.5",
                  requiredOn ? "text-success" : "text-warning",
                )}
              >
                <span aria-hidden="true" className="size-1.5 rounded-full bg-current" />
                {requiredOn ? "Connected" : "Not connected"}
              </span>
            </dd>
          </>
        ) : null}
        <dt className="text-muted-foreground">Made by</dt>
        <dd>Built into {APP_NAME}</dd>
      </dl>
    </aside>
  );
}
