import { useQuery } from "@tanstack/react-query";
import { cn } from "cn";
import { Link } from "@tanstack/react-router";
import { ChevronRight, CirclePause, Clock, VideoOff } from "lucide-react";
import type { ComponentType } from "react";

import type { ConferencingApp } from "../../bindings/ConferencingApp";
import { Badge } from "../../components/ui/badge";
import { Button } from "../../components/ui/button";
import { Card } from "../../components/ui/card";
import { Skeleton } from "../../components/ui/skeleton";
import { zoneName } from "../../lib/format";
import { ROUTES } from "../../lib/routes";
import { callPlace, CONFERENCING_APP } from "../integrations/conferencing";
import { hostPageQuery } from "../widget/cadence-api";
import { availabilityQuery, hostSettingsQuery } from "./host-api";
import { callLengthLabel, daysLabel } from "./hours-labels";
import { SETTINGS_SECTIONS } from "./routes";

type Problem = {
  /** Why the step matters, in the host's terms. */
  detail: string;
  icon: ComponentType<{ className?: string }>;
  /** Whether it stops people booking at all, not just joining. */
  blocksBooking: boolean;
  fix: {
    label: string;
    to:
      | typeof ROUTES.dashboardAvailability
      | typeof ROUTES.dashboardAppsInstalled
      | typeof ROUTES.dashboardSettings;
    hash?: string;
  };
};

/** What stops people from booking, or from joining a call they booked, and how to fix it. */
export function bookingPageProblems({
  paused,
  hasHours,
  place,
}: {
  paused: boolean;
  hasHours: boolean;
  place: ConferencingApp | null;
}): Problem[] {
  const problems: Problem[] = [];
  if (paused) {
    problems.push({
      detail: "Your booking page is paused, so nobody can book you.",
      icon: CirclePause,
      blocksBooking: true,
      fix: {
        label: "Take bookings again",
        to: ROUTES.dashboardSettings,
        hash: SETTINGS_SECTIONS.bookingPage,
      },
    });
  }
  if (!hasHours) {
    problems.push({
      detail: "Your link has no times to offer until you do.",
      icon: Clock,
      blocksBooking: true,
      fix: { label: "Set your hours", to: ROUTES.dashboardAvailability },
    });
  }
  if (place === null) {
    problems.push({
      detail: "People who book you get no link to join the call.",
      icon: VideoOff,
      blocksBooking: false,
      fix: { label: "Add a meeting link", to: ROUTES.dashboardAppsInstalled },
    });
  }
  return problems;
}

/** The booking page as the overview reports it, from the queries it already shares. */
function useBookingPageState() {
  const settings = useQuery(hostSettingsQuery);
  const availability = useQuery(availabilityQuery);
  const slug = settings.data?.public_slug;
  const page = useQuery({ ...hostPageQuery(slug ?? ""), enabled: Boolean(slug) });
  const rules = availability.data ?? [];
  const place = settings.data
    ? callPlace({
        app: settings.data.conferencing_app,
        calendarConnected: page.data?.calendar_connected ?? false,
        hasLink: Boolean(settings.data.meeting_link),
      })
    : null;
  // A paused page answers as not found. Without the page, Google Meet is unknown,
  // and that is the only case where "no place to join" could be wrong.
  const placeKnown =
    page.isSuccess ||
    place !== null ||
    settings.data?.conferencing_app !== CONFERENCING_APP.googleMeet;
  const pending =
    settings.isPending || availability.isPending || (Boolean(slug) && page.isPending);
  const failed = settings.isError || availability.isError;
  return {
    pending,
    failed,
    retry: () => {
      void settings.refetch();
      void availability.refetch();
    },
    problems:
      pending || failed || !settings.data
        ? []
        : bookingPageProblems({
            paused: settings.data.is_active === false,
            hasHours: rules.length > 0,
            place: placeKnown ? place : CONFERENCING_APP.googleMeet,
          }),
    summary:
      pending || failed || !settings.data
        ? null
        : [
            rules.length > 0 ? `${daysLabel(rules)}, ${callLengthLabel(rules)}` : null,
            zoneName(settings.data.timezone),
            place === CONFERENCING_APP.googleMeet
              ? "Google Meet links"
              : place === CONFERENCING_APP.customLink
                ? "Your own meeting link"
                : null,
          ]
            .filter(Boolean)
            .join(" · "),
  };
}

/** Only when something is left to do: each step, why it matters, and a link straight to it. */
export function SetupProblems() {
  const { problems } = useBookingPageState();
  if (problems.length === 0) return null;
  const blocked = problems.some((problem) => problem.blocksBooking);
  return (
    <Card className="gap-0 overflow-hidden py-0">
      <div className="flex items-start justify-between gap-4 px-4 py-4 sm:px-5">
        <div className="min-w-0">
          <h2 className="font-semibold">Next steps</h2>
          <p className="text-sm text-muted-foreground">
            {blocked
              ? "Nobody can book you until these are done."
              : "People can book you. This makes their calls run smoothly."}
          </p>
        </div>
        <Badge variant="secondary" className="shrink-0">
          {problems.length} to do
        </Badge>
      </div>
      <ul className="divide-y border-t">
        {problems.map((problem) => {
          const Icon = problem.icon;
          return (
            <li key={problem.fix.label}>
              <Link
                to={problem.fix.to}
                hash={problem.fix.hash}
                className="group flex items-center gap-3 px-4 py-3 outline-none transition-colors hover:bg-muted/50 focus-visible:bg-muted/50 active:bg-muted focus-visible:ring-[3px] focus-visible:ring-ring/50 focus-visible:ring-inset sm:px-5"
              >
                <span
                  className={cn(
                    "grid size-9 shrink-0 place-items-center rounded-lg border",
                    problem.blocksBooking
                      ? "border-warning/40 bg-warning/10 text-warning"
                      : "bg-muted text-muted-foreground",
                  )}
                >
                  <Icon className="size-4" />
                </span>
                <span className="min-w-0 flex-1">
                  <span className="block font-medium">{problem.fix.label}</span>
                  <span className="block text-sm text-muted-foreground">
                    {problem.detail}
                  </span>
                </span>
                <ChevronRight className="size-4 shrink-0 text-muted-foreground transition-transform group-hover:translate-x-0.5 motion-reduce:transition-none" />
              </Link>
            </li>
          );
        })}
      </ul>
    </Card>
  );
}

/** How the booking page is set up, in one quiet line under the calls. */
export function SetupSummary() {
  const { pending, failed, retry, summary } = useBookingPageState();
  return (
    <footer className="flex flex-wrap items-center justify-between gap-x-4 gap-y-2 border-t pt-4 text-sm text-muted-foreground">
      {pending ? (
        <Skeleton className="h-5 w-64" />
      ) : failed ? (
        <p className="text-destructive">Your booking page setup did not load.</p>
      ) : (
        <p className="min-w-0">{summary}</p>
      )}
      <div className="flex flex-wrap gap-1">
        {failed ? (
          <Button variant="outline" size="sm" onClick={retry}>
            Try again
          </Button>
        ) : null}
        <Button variant="ghost" size="sm" asChild>
          <Link to={ROUTES.dashboardAvailability}>Edit hours</Link>
        </Button>
        <Button variant="ghost" size="sm" asChild>
          <Link to={ROUTES.dashboardSettings}>Settings</Link>
        </Button>
      </div>
    </footer>
  );
}
