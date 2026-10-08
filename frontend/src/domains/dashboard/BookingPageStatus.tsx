import { useQuery, type UseQueryResult } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import { AlertTriangle, Copy, ExternalLink } from "lucide-react";
import type { ReactNode } from "react";

import type { AvailabilityRule } from "../../bindings/AvailabilityRule";
import { Button } from "../../components/ui/button";
import { Card } from "../../components/ui/card";
import { Skeleton } from "../../components/ui/skeleton";
import { api } from "../../lib/api";
import { weekdayName } from "../../lib/format";
import { QUERY_KEYS } from "../../lib/query-keys";
import { bookingPath, ROUTES } from "../../lib/routes";
import { getBookingPage, getHostSettings } from "../widget/cadence-api";
import { copyBookingLink } from "./booking-link";
import { SETTINGS_SECTIONS } from "./routes";

const WEEK_DAYS = 7;

const shortDay = (weekday: number) => weekdayName(weekday).slice(0, 3);

/** "Mon to Fri, Sun" for the weekdays that have hours. */
function daysLabel(rules: AvailabilityRule[]): string {
  const days = [...new Set(rules.map((rule) => rule.weekday))].sort((a, b) => a - b);
  if (days.length === WEEK_DAYS) return "Every day";
  const runs: number[][] = [];
  for (const day of days) {
    const run = runs.at(-1);
    if (run && run.at(-1) === day - 1) run.push(day);
    else runs.push([day]);
  }
  return runs
    .map((run) => {
      const [first = 0] = run;
      const last = run.at(-1) ?? first;
      if (first === last) return shortDay(first);
      return `${shortDay(first)}${run.length === 2 ? ", " : " to "}${shortDay(last)}`;
    })
    .join(", ");
}

/** "30 min calls", or "20 and 30 min calls" when the days differ. */
function callLengthLabel(rules: AvailabilityRule[]): string {
  const lengths = [...new Set(rules.map((rule) => rule.slot_minutes))].sort(
    (a, b) => a - b,
  );
  return `${lengths.join(" and ")} min calls`;
}

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
  to: typeof ROUTES.dashboardAvailability | typeof ROUTES.dashboardSettings;
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
  const settings = useQuery({
    queryKey: QUERY_KEYS.hostSettings,
    queryFn: getHostSettings,
  });
  const availability = useQuery({
    queryKey: QUERY_KEYS.availability,
    queryFn: () => api<AvailabilityRule[]>("/api/availability"),
  });
  const slug = settings.data?.public_slug;
  const page = useQuery({
    queryKey: QUERY_KEYS.bookingPage(slug ?? ""),
    queryFn: () => getBookingPage({ data: slug ?? "" }),
    enabled: Boolean(slug),
  });
  // Where calls happen needs both the saved link and the calendar state.
  const calls = settings.isError ? settings : page;

  return (
    <Card className="gap-0 overflow-hidden py-0">
      <h2 className="border-b px-5 py-3 font-semibold">Your booking page</h2>
      <dl className="divide-y">
        <Row label="Link" query={settings}>
          {() => ({
            value: `${window.location.host}${bookingPath(slug ?? "")}`,
            actions: (
              <>
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
            value: settings.data?.timezone.replaceAll("_", " "),
            actions: (
              <FixLink to={ROUTES.dashboardSettings} hash={SETTINGS_SECTIONS.time}>
                Change
              </FixLink>
            ),
          })}
        </Row>
        <Row label="Calls" query={calls}>
          {() => {
            const hasCalendar = page.data?.calendar_connected ?? false;
            const hasLink = Boolean(settings.data?.meeting_link);
            const none = !hasCalendar && !hasLink;
            return {
              problem: none,
              value: hasCalendar
                ? "A new Google Meet link for each call"
                : hasLink
                  ? "Your own meeting link"
                  : "No place to join, so people who book get no link",
              actions: (
                <FixLink
                  to={ROUTES.dashboardSettings}
                  hash={SETTINGS_SECTIONS.calls}
                  primary={none}
                >
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
