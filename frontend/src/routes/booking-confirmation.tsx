import { createFileRoute, Link } from "@tanstack/react-router";
import { CalendarCheck, CalendarX, Video } from "lucide-react";

import { Button } from "../components/ui/button";
import { PublicLayout } from "../domains/shared/PublicLayout";
import { APP_NAME } from "../lib/config";
import { formatClock } from "../lib/format";
import { ROUTES } from "../lib/routes";

const MEET_ORIGIN = "https://meet.google.com/";

function text(value: unknown, pattern: RegExp): string | undefined {
  return typeof value === "string" && pattern.test(value) ? value : undefined;
}

export const Route = createFileRoute("/booking-confirmation")({
  // The values arrive in the URL, so nothing is shown unless it has the expected shape.
  validateSearch: (search) => ({
    date: text(search.date, /^\d{4}-\d{2}-\d{2}$/),
    time: text(search.time, /^\d{1,2}:\d{2}/),
    tz: text(search.tz, /^[\w+-]+(\/[\w+-]+)*$/),
    meet:
      typeof search.meet === "string" && search.meet.startsWith(MEET_ORIGIN)
        ? search.meet
        : undefined,
  }),
  head: () => ({ meta: [{ title: `Booking confirmed | ${APP_NAME}` }] }),
  component: BookingConfirmationPage,
});

function BookingConfirmationPage() {
  const { date, time, tz, meet } = Route.useSearch();

  if (!date || !time) {
    return (
      <PublicLayout>
        <section className="mx-auto flex max-w-lg flex-col items-center gap-4 px-4 py-20 text-center">
          <CalendarX className="size-8 text-muted-foreground" />
          <h1 className="font-display text-3xl font-semibold tracking-tight">
            No booking to show
          </h1>
          <p className="text-muted-foreground">
            This page shows a booking right after it is made. Booked already? The details
            are in your confirmation email.
          </p>
          <Button asChild>
            <Link to={ROUTES.home}>Book a call</Link>
          </Button>
        </section>
      </PublicLayout>
    );
  }

  const day = new Intl.DateTimeFormat("en", {
    weekday: "long",
    day: "numeric",
    month: "long",
    year: "numeric",
    timeZone: "UTC",
  }).format(new Date(`${date}T00:00:00Z`));

  return (
    <PublicLayout>
      <section className="mx-auto max-w-lg px-4 py-14 sm:py-20">
        <span className="flex size-11 items-center justify-center rounded-full bg-success/10 text-success">
          <CalendarCheck className="size-5" />
        </span>
        <h1 className="mt-5 font-display text-3xl font-semibold tracking-tight sm:text-4xl">
          Your call is booked
        </h1>
        <p className="mt-3 text-muted-foreground">
          A confirmation with the video-call details is on its way to your inbox.
        </p>

        <dl className="mt-8 divide-y rounded-xl border bg-card text-sm">
          <div className="flex justify-between gap-4 px-4 py-3">
            <dt className="text-muted-foreground">Date</dt>
            <dd className="text-right font-medium">{day}</dd>
          </div>
          <div className="flex justify-between gap-4 px-4 py-3">
            <dt className="text-muted-foreground">Time</dt>
            <dd className="text-right font-medium tabular-nums">
              {formatClock(time, false)}
            </dd>
          </div>
          {tz ? (
            <div className="flex justify-between gap-4 px-4 py-3">
              <dt className="text-muted-foreground">Timezone</dt>
              <dd className="text-right font-medium">{tz.replaceAll("_", " ")}</dd>
            </div>
          ) : null}
        </dl>

        <div className="mt-8 flex flex-wrap gap-3">
          {meet ? (
            <Button asChild>
              <a href={meet} target="_blank" rel="noreferrer">
                <Video />
                Open Google Meet
              </a>
            </Button>
          ) : null}
          <Button variant="outline" asChild>
            <Link to={ROUTES.home}>Back to {APP_NAME}</Link>
          </Button>
        </div>
      </section>
    </PublicLayout>
  );
}
