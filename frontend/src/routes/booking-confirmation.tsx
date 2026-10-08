import { createFileRoute, Link } from "@tanstack/react-router";
import { cn } from "cn";
import { CalendarX, Video } from "lucide-react";

import { Button } from "../components/ui/button";
import { PublicLayout } from "../domains/shared/PublicLayout";
import { PHASE_STYLES } from "../domains/widget/SlotStep";
import { callEnd, clockLabel, dayPhase, zonedTime } from "../domains/widget/zoned-time";
import { APP_NAME, NOINDEX_META } from "../lib/config";
import { zoneName } from "../lib/format";
import { ROUTES } from "../lib/routes";
import { searchInstant, searchMinutes, searchText, searchTimezone } from "../lib/search";

const MEET_ORIGIN = "https://meet.google.com/";
const MAX_CALL_MINUTES = 240;
/** The strip shows the hour before the call and the two after it starts. */
const STRIP_HOURS = 4;
const STRIP_MINUTES = STRIP_HOURS * 60;

export const Route = createFileRoute("/booking-confirmation")({
  // The values arrive in the URL, so nothing is shown unless it has the expected shape.
  // Only a Google Meet address can become a button: a crafted URL must not be able
  // to put an arbitrary "join" link on this page.
  validateSearch: (search) => ({
    start: searchInstant(search.start),
    minutes: searchMinutes(search.minutes, MAX_CALL_MINUTES),
    tz: searchTimezone(search.tz),
    // The clock the visitor chose on the booking card. This page is rendered on
    // the server, which cannot know the device's own style.
    h12: search.h12 === true,
    meet:
      typeof search.meet === "string" && search.meet.startsWith(MEET_ORIGIN)
        ? search.meet
        : undefined,
    // Whether the booking has a place to meet. A host's own link is only sent by email.
    link: search.link === true,
    slug: searchText(search.slug, /^[a-z0-9]+(-[a-z0-9]+)*$/),
  }),
  head: () => ({ meta: [{ title: `Booking confirmed | ${APP_NAME}` }, NOINDEX_META] }),
  component: BookingConfirmationPage,
});

/** The booked call as a block on its stretch of the day. */
function BookedStrip({
  start,
  minutes,
  timeZone,
}: {
  start: Date;
  minutes: number;
  timeZone: string;
}) {
  const at = zonedTime(start, timeZone);
  const firstHour = at.hour - 1;
  const styles = PHASE_STYLES[dayPhase(at.hour)];
  const left = ((60 + at.minute) / STRIP_MINUTES) * 100;
  const width = Math.min((minutes / STRIP_MINUTES) * 100, 100 - left);

  return (
    <div aria-hidden="true" className="rounded-xl border bg-card px-4 pt-3 pb-4">
      <div className="grid grid-cols-4">
        {Array.from({ length: STRIP_HOURS }, (_, index) => (
          <div key={index}>
            <p className="font-display font-narrow text-sm font-medium text-muted-foreground tabular-nums">
              {String((firstHour + index + 24) % 24).padStart(2, "0")}:00
            </p>
            <div className="relative mt-1 h-2.5 border-t border-foreground/25">
              <span className="absolute top-0 left-0 h-2.5 w-px bg-foreground/40" />
              <span className="absolute top-0 left-1/2 h-1.5 w-px bg-foreground/25" />
            </div>
          </div>
        ))}
      </div>
      <div className="relative mt-2 h-9">
        <span
          style={{ left: `${left}%`, width: `${width}%` }}
          className={cn(
            "absolute inset-y-0 min-w-6 rounded-md motion-safe:animate-block-drop",
            styles.rail,
          )}
        />
      </div>
    </div>
  );
}

function BookingConfirmationPage() {
  const { start, minutes, tz, h12, meet, link, slug } = Route.useSearch();

  if (!start || !minutes || !tz) {
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
            <Link to={ROUTES.home}>Go to the home page</Link>
          </Button>
        </section>
      </PublicLayout>
    );
  }

  const startsAt = new Date(start);
  const day = new Intl.DateTimeFormat("en", {
    weekday: "long",
    day: "numeric",
    month: "long",
    year: "numeric",
    timeZone: tz,
  }).format(startsAt);

  return (
    <PublicLayout>
      <section className="mx-auto max-w-lg px-4 py-14 sm:py-20">
        <h1 className="font-display font-wide text-4xl leading-none font-bold tracking-tight sm:text-5xl">
          Your call is booked
        </h1>
        <p className="mt-4 text-muted-foreground">
          {link
            ? "A confirmation with the link to join is on its way to your inbox."
            : "A confirmation is on its way to your inbox."}
        </p>

        <div className="mt-8">
          <BookedStrip start={startsAt} minutes={minutes} timeZone={tz} />
        </div>

        <dl className="mt-4 divide-y rounded-xl border bg-card text-sm">
          <div className="flex justify-between gap-4 px-4 py-3">
            <dt className="text-muted-foreground">Date</dt>
            <dd className="text-right font-medium">{day}</dd>
          </div>
          <div className="flex justify-between gap-4 px-4 py-3">
            <dt className="text-muted-foreground">Time</dt>
            <dd className="text-right font-medium tabular-nums">
              {clockLabel(startsAt, tz, h12)} to{" "}
              {clockLabel(callEnd(start, minutes), tz, h12)}
            </dd>
          </div>
          <div className="flex justify-between gap-4 px-4 py-3">
            <dt className="text-muted-foreground">Timezone</dt>
            <dd className="text-right font-medium">{zoneName(tz)}</dd>
          </div>
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
          {slug ? (
            <Button variant="outline" asChild>
              <Link to={ROUTES.bookingPage} params={{ slug }}>
                Book another time
              </Link>
            </Button>
          ) : null}
        </div>
      </section>
    </PublicLayout>
  );
}
