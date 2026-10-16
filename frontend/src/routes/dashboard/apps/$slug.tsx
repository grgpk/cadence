import { useQuery } from "@tanstack/react-query";
import { createFileRoute, Link, notFound } from "@tanstack/react-router";
import { ArrowLeft } from "lucide-react";

import { Badge } from "../../../components/ui/badge";
import { Button } from "../../../components/ui/button";
import { Skeleton } from "../../../components/ui/skeleton";
import { hostSettingsQuery } from "../../../domains/dashboard/host-api";
import { AppLogo, AppStatus } from "../../../domains/integrations/app-card";
import { CallFlow } from "../../../domains/integrations/call-flow";
import {
  APP_CATEGORY,
  APP_SLUG,
  type AppEntry,
  CATEGORY_INFO,
  findApp,
  isInstalled,
} from "../../../domains/integrations/catalog";
import { callPlace } from "../../../domains/integrations/conferencing";
import { ConferencingForm } from "../../../domains/integrations/conferencing-form";
import { GoogleCalendarConnection } from "../../../domains/integrations/google-calendar-connection";
import { ErrorState } from "../../../domains/shared/error-state";
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
  if (!app) return null;
  return (
    <>
      <Button variant="ghost" size="sm" className="-ml-2 w-fit" asChild>
        <Link to={ROUTES.dashboardApps}>
          <ArrowLeft />
          App store
        </Link>
      </Button>
      <div className="grid gap-8 lg:grid-cols-[minmax(0,1.1fr)_minmax(0,1fr)] lg:items-start">
        <Preview app={app} />
        <About app={app} />
      </div>
    </>
  );
}

/** A large mark for the app, with what it does to every booking. */
function Preview({ app }: { app: AppEntry }) {
  const Icon = app.icon;
  return (
    <div className="grid min-h-72 place-items-center rounded-2xl border bg-muted/50 p-8">
      <div className="grid justify-items-center gap-4 text-center">
        <span className="grid size-24 place-items-center rounded-3xl border bg-card shadow-sm">
          <Icon className="size-12 text-muted-foreground" />
        </span>
        <p className="max-w-xs text-sm text-muted-foreground">{app.summary}</p>
      </div>
    </div>
  );
}

function About({ app }: { app: AppEntry }) {
  const settings = useQuery(hostSettingsQuery);
  const slug = settings.data?.public_slug;
  const page = useQuery({ ...bookingPageQuery(slug ?? ""), enabled: Boolean(slug) });
  const calendarConnected = page.data?.calendar_connected ?? false;
  const category = CATEGORY_INFO[app.category];

  return (
    <div className="grid gap-6">
      <div className="flex items-center gap-4">
        <AppLogo app={app} size="lg" />
        <div className="min-w-0">
          <h1 className="font-display font-wide text-3xl font-bold tracking-tight">
            {app.name}
          </h1>
          <p className="mt-1.5 flex flex-wrap items-center gap-2 text-sm text-muted-foreground">
            <Badge variant="secondary">{category.name}</Badge>
            <span>Built into {APP_NAME}</span>
            {settings.data ? (
              <AppStatus
                app={app}
                installed={isInstalled(app.slug, settings.data, calendarConnected)}
              />
            ) : null}
          </p>
        </div>
      </div>

      <p className="leading-relaxed">{app.description}</p>

      <dl className="grid grid-cols-2 gap-4 text-sm">
        <div>
          <dt className="font-semibold">Pricing</dt>
          <dd className="text-muted-foreground">Free</dd>
        </div>
        <div>
          <dt className="font-semibold">Category</dt>
          <dd className="text-muted-foreground">{category.description}</dd>
        </div>
      </dl>

      {app.available ? (
        <section className="grid gap-4 border-t pt-6">
          <h2 className="font-semibold">Set up</h2>
          {settings.isPending || (slug && page.isPending) ? (
            <Skeleton className="h-24 w-full rounded-xl" />
          ) : settings.isError ? (
            <ErrorState
              title="Your apps did not load"
              error={settings.error}
              onRetry={() => void settings.refetch()}
            />
          ) : app.slug === APP_SLUG.googleCalendar ? (
            <GoogleCalendarConnection slug={settings.data.public_slug} />
          ) : app.category === APP_CATEGORY.conferencing ? (
            <>
              <CallFlow
                calendarConnected={calendarConnected}
                place={callPlace({
                  app: settings.data.conferencing_app,
                  calendarConnected,
                  hasLink: Boolean(settings.data.meeting_link),
                })}
              />
              <ConferencingForm
                saved={settings.data}
                calendarConnected={calendarConnected}
              />
            </>
          ) : null}
        </section>
      ) : (
        <p className="border-t pt-6 text-sm text-muted-foreground">
          This app is on the way. Until then, use{" "}
          <Link
            to={ROUTES.dashboardAppsInstalled}
            className="font-medium text-foreground underline underline-offset-4"
          >
            the apps you have
          </Link>
          .
        </p>
      )}
    </div>
  );
}
