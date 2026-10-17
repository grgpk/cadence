import { useQuery } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";

import { Button } from "../../components/ui/button";
import { Card } from "../../components/ui/card";
import { Skeleton } from "../../components/ui/skeleton";
import { hourLabel, weekdayName } from "../../lib/format";
import { ROUTES } from "../../lib/routes";
import { insightsQuery, showUpRate } from "../bookings/bookings";
import { ErrorState } from "../shared/error-state";

function Figure({ label, value, note }: { label: string; value: string; note?: string }) {
  return (
    <div className="grid content-start gap-1 px-5 py-4">
      <dt className="text-sm text-muted-foreground">{label}</dt>
      <dd className="text-clock text-3xl font-semibold tracking-tight">{value}</dd>
      {note ? <dd className="text-sm text-muted-foreground">{note}</dd> : null}
    </div>
  );
}

/** Business: the last 7 days, and what the last 30 say about the week. */
export function InsightsCard({ hour12 }: { hour12: boolean }) {
  const query = useQuery(insightsQuery);
  const week = query.data;
  const rate = week ? showUpRate(week.held, week.no_shows) : null;
  return (
    <Card className="gap-0 overflow-hidden py-0">
      <div className="flex items-center justify-between gap-4 border-b py-2.5 pr-3 pl-5">
        <h2 className="font-semibold">Your last 7 days</h2>
        <Button variant="ghost" size="sm" asChild>
          <Link to={ROUTES.dashboardBookings}>All bookings</Link>
        </Button>
      </div>
      {query.isError ? (
        <div className="p-5">
          <ErrorState
            title="Your week did not load"
            error={query.error}
            onRetry={() => void query.refetch()}
          />
        </div>
      ) : week ? (
        <>
          <dl className="grid divide-y sm:grid-cols-4 sm:divide-x sm:divide-y-0">
            <Figure label="Booked" value={String(week.booked)} />
            <Figure
              label="Showed up"
              value={rate === null ? "–" : `${rate}%`}
              note={
                week.held === 0
                  ? "No calls held yet"
                  : `${week.no_shows} of ${week.held} missed`
              }
            />
            <Figure
              label="Confirmed ahead"
              value={String(week.confirmed)}
              note={week.held === 0 ? undefined : `of ${week.held} held`}
            />
            <Figure
              label="Left without booking"
              value={String(week.unfinished)}
              note={
                week.unfinished > 0 ? "Send them your times from Bookings" : undefined
              }
            />
          </dl>
          {week.busiest || week.sources.length > 0 ? (
            <p className="border-t px-5 py-3 text-sm text-muted-foreground">
              {week.busiest
                ? `Your busiest time over 30 days is ${weekdayName(week.busiest.weekday)} at ${hourLabel(week.busiest.hour, hour12)}. `
                : null}
              {week.sources[0]
                ? `Most bookings came from ${week.sources[0].source}.`
                : null}
            </p>
          ) : null}
        </>
      ) : (
        <div className="grid gap-3 p-5" role="status" aria-label="Loading your week">
          <Skeleton className="h-16 w-full" />
        </div>
      )}
    </Card>
  );
}
