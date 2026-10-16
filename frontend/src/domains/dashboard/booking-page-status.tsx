import { useQuery } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import { AlertTriangle } from "lucide-react";

import type { ConferencingApp } from "../../bindings/ConferencingApp";
import { Alert, AlertDescription, AlertTitle } from "../../components/ui/alert";
import { Button } from "../../components/ui/button";
import { Skeleton } from "../../components/ui/skeleton";
import { zoneName } from "../../lib/format";
import { ROUTES } from "../../lib/routes";
import { callPlace, CONFERENCING_APP } from "../integrations/conferencing";
import { bookingPageQuery } from "../widget/cadence-api";
import { availabilityQuery, hostSettingsQuery } from "./host-api";
import { callLengthLabel, daysLabel } from "./hours-labels";
import { SETTINGS_SECTIONS } from "./routes";

type Problem = {
  message: string;
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
      message: "Your booking page is paused, so nobody can book you.",
      fix: {
        label: "Take bookings again",
        to: ROUTES.dashboardSettings,
        hash: SETTINGS_SECTIONS.rules,
      },
    });
  }
  if (!hasHours) {
    problems.push({
      message: "You have no hours set, so your link has no times to offer.",
      fix: { label: "Set your hours", to: ROUTES.dashboardAvailability },
    });
  }
  if (place === null) {
    problems.push({
      message: "There is no place to join, so people who book get no call link.",
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
  const page = useQuery({ ...bookingPageQuery(slug ?? ""), enabled: Boolean(slug) });
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

/** Only when something is wrong: what it is, and the one place that fixes it. */
export function SetupProblems() {
  const { problems } = useBookingPageState();
  if (problems.length === 0) return null;
  return (
    <Alert className="border-warning/50 bg-warning/10">
      <AlertTriangle className="text-warning" />
      <AlertTitle>
        {problems.length === 1
          ? "Your booking page needs one fix"
          : `Your booking page needs ${problems.length} fixes`}
      </AlertTitle>
      <AlertDescription className="mt-1 gap-2 text-foreground">
        {problems.map((problem) => (
          <div
            key={problem.message}
            className="flex w-full flex-wrap items-center justify-between gap-x-4 gap-y-2"
          >
            <p>{problem.message}</p>
            <Button size="sm" asChild>
              <Link to={problem.fix.to} hash={problem.fix.hash}>
                {problem.fix.label}
              </Link>
            </Button>
          </div>
        ))}
      </AlertDescription>
    </Alert>
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
