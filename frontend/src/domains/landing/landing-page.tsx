import { Link } from "@tanstack/react-router";
import {
  Bot,
  Code2,
  Mail,
  type LucideIcon,
  RefreshCw,
  Bell,
  Webhook,
} from "lucide-react";
import type { ReactNode } from "react";

import { Button } from "../../components/ui/button";
import { useSessionUser } from "../../lib/auth";
import { ROUTES } from "../../lib/routes";
import { GoogleCalendarIcon } from "../integrations/google-calendar-icon";
import { GoogleMeetIcon } from "../integrations/google-meet-icon";
import { PromiseList } from "../pricing/promises";
import { PublicLayout } from "../shared/public-layout";
import { BookingDemo } from "./booking-demo";
import { HOW_IT_WORKS_ID } from "./section-ids";
import { HoursPreview, LinkPreview, TimesPreview } from "./step-previews";

const CONTAINER = "mx-auto max-w-[82rem] px-4 sm:px-6";
const SECTION = "border-t py-16 sm:py-24";
const H2 =
  "font-display font-wide text-3xl leading-[1.05] font-bold tracking-tight sm:text-4xl";

const STEPS = [
  {
    Preview: HoursPreview,
    title: "Set your hours",
    body: "Add the weekly hours when you take calls, in your own timezone. Change them whenever your week changes.",
  },
  {
    Preview: LinkPreview,
    title: "Share your link",
    body: "You get one link. Send it instead of trading emails about who is free when.",
  },
  {
    Preview: TimesPreview,
    title: "Get booked",
    body: "People pick a time in their timezone. Busy times on your Google Calendar never show up.",
  },
];

/** What happens to a booking, in the order it happens. */
const AFTER_BOOKING: { icon: ReactNode; title: string; body: string }[] = [
  {
    icon: <Mail className="size-4" />,
    title: "Both of you get an email",
    body: "Each confirmation shows the time in that person's own timezone.",
  },
  {
    icon: <GoogleCalendarIcon className="size-4" />,
    title: "It goes on your calendar",
    body: "With Google Calendar connected, the call is added with a Google Meet link.",
  },
  {
    icon: <Bell className="size-4" />,
    title: "A reminder before the call",
    body: "Your guest gets a reminder and can say they are still coming.",
  },
  {
    icon: <RefreshCw className="size-4" />,
    title: "Plans change, and that is fine",
    body: "Every email has a link to reschedule or cancel. No back and forth.",
  },
];

const WORKS_WITH: { icon: ReactNode; name: string }[] = [
  { icon: <GoogleCalendarIcon className="size-5" />, name: "Google Calendar" },
  { icon: <GoogleMeetIcon className="size-5" />, name: "Google Meet" },
  { icon: <LucideGlyph icon={Code2} />, name: "Embed on your site" },
  { icon: <LucideGlyph icon={Webhook} />, name: "Webhooks" },
  { icon: <LucideGlyph icon={Bot} />, name: "AI agents over MCP" },
];

function LucideGlyph({ icon: Icon }: { icon: LucideIcon }) {
  return <Icon className="size-5 text-muted-foreground" aria-hidden />;
}

function SignUpButton() {
  return (
    <Button size="lg" asChild>
      <Link to={ROUTES.register}>Create your account</Link>
    </Button>
  );
}

export function LandingPage() {
  // Sign-up is for visitors; the header already leads a signed-in host home.
  const signedIn = useSessionUser() !== null;
  return (
    <PublicLayout>
      <section className="pt-14 pb-16 sm:pt-24 sm:pb-24">
        <div className={CONTAINER}>
          <div className="grid items-end gap-x-12 gap-y-7 lg:grid-cols-[minmax(0,7fr)_minmax(0,5fr)]">
            <h1 className="motion-safe:animate-rise font-display font-wide text-5xl leading-[0.98] font-bold tracking-tight sm:text-7xl">
              Your week, open for booking.
            </h1>
            <div className="motion-safe:animate-rise motion-safe:[animation-delay:120ms]">
              <p className="max-w-md text-lg text-muted-foreground">
                Share one link. People pick a time that works for them, and it lands on
                your calendar.
              </p>
              <div className="mt-6 grid gap-2 min-[25rem]:flex min-[25rem]:flex-wrap">
                {signedIn ? null : <SignUpButton />}
                <Button size="lg" variant="ghost" asChild>
                  <Link to={ROUTES.pricing}>See pricing</Link>
                </Button>
              </div>
            </div>
          </div>
          <div className="mt-12 motion-safe:animate-rise motion-safe:[animation-delay:240ms] sm:mt-16">
            <BookingDemo />
          </div>
        </div>
      </section>

      <section id={HOW_IT_WORKS_ID} className={`${SECTION} scroll-mt-14`}>
        <div className={CONTAINER}>
          <h2 className={H2}>How it works</h2>
          <ol className="mt-10 grid gap-x-8 gap-y-12 md:grid-cols-3">
            {STEPS.map(({ Preview, title, body }, index) => (
              <li key={title}>
                <Preview />
                <div className="mt-5 flex items-baseline gap-3">
                  <span className="text-clock text-3xl font-semibold text-dusk">
                    {index + 1}
                  </span>
                  <h3 className="text-lg font-semibold">{title}</h3>
                </div>
                <p className="mt-1.5 max-w-sm text-muted-foreground">{body}</p>
              </li>
            ))}
          </ol>
        </div>
      </section>

      <section className={SECTION}>
        <div
          className={`${CONTAINER} grid gap-10 lg:grid-cols-[minmax(0,5fr)_minmax(0,7fr)]`}
        >
          <div>
            <h2 className={H2}>After someone books</h2>
            <p className="mt-4 max-w-sm text-lg text-muted-foreground">
              You do not have to do anything. This is what happens next.
            </p>
          </div>
          <ol className="grid gap-8">
            {AFTER_BOOKING.map(({ icon, title, body }) => (
              <li
                key={title}
                // A rail from each step down to the next; the last one ends the line.
                className="relative flex gap-4 not-last:after:absolute not-last:after:top-9 not-last:after:-bottom-8 not-last:after:left-[1.0625rem] not-last:after:w-px not-last:after:bg-border"
              >
                <span
                  aria-hidden
                  className="relative flex size-9 shrink-0 items-center justify-center rounded-full border bg-background text-foreground"
                >
                  {icon}
                </span>
                <div className="pt-1.5">
                  <h3 className="font-semibold">{title}</h3>
                  <p className="mt-1 max-w-md text-muted-foreground">{body}</p>
                </div>
              </li>
            ))}
          </ol>
        </div>
      </section>

      <section className={SECTION}>
        <div className={CONTAINER}>
          <h2 className={H2}>Works with what you use</h2>
          <ul className="mt-8 flex flex-wrap gap-x-8 gap-y-4">
            {WORKS_WITH.map(({ icon, name }) => (
              <li key={name} className="flex items-center gap-2.5 font-medium">
                {icon}
                {name}
              </li>
            ))}
          </ul>
          <p className="mt-8 max-w-xl text-muted-foreground">
            Coming from Calendly? Bring your event types, weekly hours and days off over
            in a minute after you sign up.
          </p>
        </div>
      </section>

      <section className={SECTION}>
        <div className={CONTAINER}>
          <div className="flex flex-wrap items-end justify-between gap-x-8 gap-y-4">
            <h2 className={`${H2} max-w-2xl`}>What every plan promises</h2>
            <Button variant="outline" asChild>
              <Link to={ROUTES.pricing}>See pricing</Link>
            </Button>
          </div>
          <div className="mt-10">
            <PromiseList row />
          </div>
        </div>
      </section>

      <section className="border-t bg-foreground py-16 text-background sm:py-24">
        <div
          className={`${CONTAINER} flex flex-wrap items-center justify-between gap-x-8 gap-y-6`}
        >
          <h2 className={`${H2} max-w-2xl`}>Open your week in a few minutes.</h2>
          {signedIn ? null : (
            <Button size="lg" variant="secondary" asChild>
              <Link to={ROUTES.register}>Create your account</Link>
            </Button>
          )}
        </div>
      </section>
    </PublicLayout>
  );
}
