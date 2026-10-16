import { createFileRoute, Link } from "@tanstack/react-router";
import { CalendarPlus, CalendarX, Download, Video } from "lucide-react";

import { Button } from "../components/ui/button";
import { PublicLayout } from "../domains/shared/public-layout";
import { BookedStrip } from "../domains/bookings/booked-strip";
import { callEnd, clockLabel } from "../lib/zoned-time";
import { googleCalendarUrl, icsHref, type CalendarEvent } from "../lib/calendar-links";
import { APP_NAME, NOINDEX_META } from "../lib/config";
import { formatLongDay, zoneName } from "../lib/format";
import { ROUTES } from "../lib/routes";
import { searchInstant, searchMinutes, searchText, searchTimezone } from "../lib/search";

const MEET_ORIGIN = "https://meet.google.com/";
const MAX_CALL_MINUTES = 240;

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
    // Shown as plain text only, so any short name is safe.
    host: searchText(search.host, /^.{1,120}$/),
    // The private link to change or cancel this call.
    manage: searchText(search.manage, /^[0-9a-f]{16,128}$/),
  }),
  head: () => ({ meta: [{ title: `Booking confirmed | ${APP_NAME}` }, NOINDEX_META] }),
  component: BookingConfirmationPage,
});

function BookingConfirmationPage() {
  const { start, minutes, tz, h12, meet, link, slug, host, manage } = Route.useSearch();

  if (!start || !minutes || !tz) {
    return (
      <PublicLayout>
        <section className="mx-auto flex max-w-lg flex-col items-center gap-4 px-4 py-20 text-center">
          <CalendarX className="size-8 text-muted-foreground" />
          <h1 className="font-display font-wide text-3xl font-bold tracking-tight">
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
  const day = formatLongDay(startsAt, tz);
  const timeRange = `${clockLabel(startsAt, tz, h12)} to ${clockLabel(callEnd(start, minutes), tz, h12)}`;
  const event: CalendarEvent = {
    title: host ? `Call with ${host}` : `${APP_NAME} call`,
    start: startsAt,
    end: callEnd(start, minutes),
    location: meet,
    uid: `${start}-${slug ?? "call"}@cadence`,
  };

  return (
    <PublicLayout accountLinks={false}>
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
          <BookedStrip
            start={startsAt}
            minutes={minutes}
            timeZone={tz}
            hour12={h12}
            label={timeRange}
          />
        </div>

        <dl className="mt-4 divide-y rounded-xl border bg-card text-sm">
          {host ? (
            <div className="flex justify-between gap-4 px-4 py-3">
              <dt className="text-muted-foreground">With</dt>
              <dd className="min-w-0 text-right font-medium break-words">{host}</dd>
            </div>
          ) : null}
          <div className="flex justify-between gap-4 px-4 py-3">
            <dt className="text-muted-foreground">Date</dt>
            <dd className="text-right font-medium">{day}</dd>
          </div>
          <div className="flex justify-between gap-4 px-4 py-3">
            <dt className="text-muted-foreground">Time</dt>
            <dd className="text-right text-clock text-[0.9375rem] font-medium">
              {timeRange}
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
                Join call
              </a>
            </Button>
          ) : null}
          <Button variant={meet ? "outline" : "default"} asChild>
            <a href={googleCalendarUrl(event)} target="_blank" rel="noreferrer">
              <CalendarPlus />
              Add to Google Calendar
            </a>
          </Button>
          <Button variant="outline" asChild>
            <a href={icsHref(event)} download="cadence-call.ics">
              <Download />
              Download .ics
            </a>
          </Button>
        </div>
        <p className="mt-6 text-sm text-muted-foreground">
          {manage ? (
            <>
              Plans changed?{" "}
              <Link
                to={ROUTES.manageBooking}
                params={{ token: manage }}
                className="font-medium text-foreground underline underline-offset-4"
              >
                Reschedule or cancel
              </Link>
              . The same link is in your email.
            </>
          ) : slug ? (
            <Link
              to={ROUTES.bookingPage}
              params={{ slug }}
              className="font-medium text-foreground underline underline-offset-4"
            >
              Book another time
            </Link>
          ) : null}
        </p>
      </section>
    </PublicLayout>
  );
}
