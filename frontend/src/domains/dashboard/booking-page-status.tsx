import { useQuery, type UseQueryResult } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import { AlertTriangle, Copy, ExternalLink } from "lucide-react";
import type { ReactNode } from "react";

import { Button } from "../../components/ui/button";
import { Card } from "../../components/ui/card";
import { Skeleton } from "../../components/ui/skeleton";
import { zoneName } from "../../lib/format";
import { bookingPath, ROUTES } from "../../lib/routes";
import { callPlace, CONFERENCING_APP } from "../integrations/conferencing";
import { bookingPageQuery } from "../widget/cadence-api";
import { copyBookingLink } from "./booking-link";
import { availabilityQuery, hostSettingsQuery } from "./host-api";
import { callLengthLabel, daysLabel } from "./hours-labels";
import { SETTINGS_SECTIONS } from "./routes";

function Row({
  label,
  query,
  children,
}: {
  label: string;
  /** What the row reads from. The row shows its loading and failure itself. */
  query: Pick<UseQueryResult, "isPending" | "isError" | "refetch">;
  children: () => { value: ReactNode; problem?: boolean; actions: ReactNode };
}) {
  const row = query.isPending || query.isError ? null : children();
  return (
    <div className="grid gap-x-4 gap-y-2 px-5 py-3.5 sm:grid-cols-[6rem_minmax(0,1fr)_auto] sm:items-center">
      <dt className="text-sm text-muted-foreground">{label}</dt>
      <dd className="flex min-w-0 items-center gap-2 text-sm font-medium">
        {query.isPending ? (
          <Skeleton className="h-5 w-48" />
        ) : row ? (
          <>
            {row.problem ? (
              <AlertTriangle className="size-4 shrink-0 text-warning" />
            ) : null}
            <span className="min-w-0 truncate">{row.value}</span>
          </>
        ) : (
          <span className="font-normal text-destructive">Did not load</span>
        )}
      </dd>
      <dd className="flex flex-wrap gap-2">
        {row ? (
          row.actions
        ) : query.isError ? (
          <Button variant="outline" size="sm" onClick={() => void query.refetch()}>
            Try again
          </Button>
        ) : null}
      </dd>
    </div>
  );
}

function FixLink({
  to,
  hash,
  primary = false,
  children,
}: {
  to:
    | typeof ROUTES.dashboardAvailability
    | typeof ROUTES.dashboardIntegrations
    | typeof ROUTES.dashboardSettings;
  hash?: string;
  /** The row has a problem and this is how to fix it. */
  primary?: boolean;
  children: ReactNode;
}) {
  return (
    <Button variant={primary ? "default" : "outline"} size="sm" asChild>
      <Link to={to} hash={hash}>
        {children}
      </Link>
    </Button>
  );
}

/** Everything a booking page needs to work, with its current state and where to change it. */
export function BookingPageStatus() {
  const settings = useQuery(hostSettingsQuery);
  const availability = useQuery(availabilityQuery);
  const slug = settings.data?.public_slug;
  const paused = settings.data?.is_active === false;
  const page = useQuery({ ...bookingPageQuery(slug ?? ""), enabled: Boolean(slug) });
  // Where calls happen needs both the saved link and the calendar state.
  const calls = settings.isError ? settings : page;

  return (
    <Card className="gap-0 overflow-hidden py-0">
      <h2 className="border-b px-5 py-3.5 font-semibold">Your booking page</h2>
      <dl className="divide-y">
        <Row label="Link" query={settings}>
          {() => ({
            problem: paused,
            value: paused
              ? "Paused: your link says you are not taking bookings"
              : `${window.location.host}${bookingPath(slug ?? "")}`,
            actions: (
              <>
                {paused ? (
                  <FixLink
                    to={ROUTES.dashboardSettings}
                    hash={SETTINGS_SECTIONS.rules}
                    primary
                  >
                    Take bookings again
                  </FixLink>
                ) : null}
                <Button
                  variant="outline"
                  size="sm"
                  onClick={() => slug && copyBookingLink(slug)}
                >
                  <Copy />
                  Copy
                </Button>
                {slug ? (
                  <Button variant="outline" size="sm" asChild>
                    <Link to={ROUTES.bookingPage} params={{ slug }} target="_blank">
                      <ExternalLink />
                      Open
                    </Link>
                  </Button>
                ) : null}
              </>
            ),
          })}
        </Row>
        <Row label="Hours" query={availability}>
          {() => {
            const rules = availability.data ?? [];
            const none = rules.length === 0;
            return {
              problem: none,
              value: none
                ? "No hours set, so your link has no times to offer"
                : `${daysLabel(rules)}, ${callLengthLabel(rules)}`,
              actions: (
                <FixLink to={ROUTES.dashboardAvailability} primary={none}>
                  {none ? "Set your hours" : "Edit hours"}
                </FixLink>
              ),
            };
          }}
        </Row>
        <Row label="Timezone" query={settings}>
          {() => ({
            value: settings.data && zoneName(settings.data.timezone),
            actions: (
              <FixLink to={ROUTES.dashboardSettings} hash={SETTINGS_SECTIONS.time}>
                Change
              </FixLink>
            ),
          })}
        </Row>
        <Row label="Calls" query={calls}>
          {() => {
            const place = settings.data
              ? callPlace({
                  app: settings.data.conferencing_app,
                  calendarConnected: page.data?.calendar_connected ?? false,
                  hasLink: Boolean(settings.data.meeting_link),
                })
              : null;
            const none = place === null;
            return {
              problem: none,
              value:
                place === CONFERENCING_APP.googleMeet
                  ? "A new Google Meet link for each call"
                  : place === CONFERENCING_APP.customLink
                    ? "Your own meeting link"
                    : "No place to join, so people who book get no link",
              actions: (
                <FixLink to={ROUTES.dashboardIntegrations} primary={none}>
                  {none ? "Add a meeting link" : "Change"}
                </FixLink>
              ),
            };
          }}
        </Row>
      </dl>
    </Card>
  );
}
