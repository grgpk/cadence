import { Link } from "@tanstack/react-router";

import { Button } from "../../components/ui/button";
import { ROUTES } from "../../lib/routes";
import { PromiseList } from "../pricing/promises";
import { PublicLayout } from "../shared/public-layout";
import { DayTape } from "./day-tape";
import { HoursPreview, LinkPreview, TimesPreview } from "./step-previews";

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
    body: "People pick a time in their timezone. Connect Google Calendar and busy times disappear and each call gets a Meet link.",
  },
];

export function LandingPage() {
  return (
    <PublicLayout>
      <section className="pt-12 pb-16 sm:pt-20 sm:pb-24">
        <div className="mx-auto max-w-[82rem] px-4 sm:px-6">
          <h1 className="max-w-4xl motion-safe:animate-rise font-display font-wide text-5xl leading-[0.98] font-bold tracking-tight sm:text-7xl">
            Your week, open for booking.
          </h1>
          <div className="mt-7 flex flex-wrap items-center gap-x-8 gap-y-5 motion-safe:animate-rise motion-safe:[animation-delay:150ms]">
            <p className="max-w-md text-lg text-muted-foreground">
              Set the hours you take calls and share one link. People pick a time, and it
              is on your calendar.
            </p>
            <div className="flex flex-wrap gap-2">
              <Button size="lg" asChild>
                <Link to={ROUTES.register}>Create your account</Link>
              </Button>
              <Button size="lg" variant="ghost" asChild>
                <Link to={ROUTES.pricing}>See pricing</Link>
              </Button>
            </div>
          </div>
        </div>
        <div className="mt-12 sm:mt-16">
          <DayTape />
        </div>
      </section>

      <section className="border-t bg-card">
        <div className="mx-auto max-w-[82rem] px-4 py-16 sm:px-6 sm:py-24">
          <h2 className="font-display font-wide text-3xl font-bold tracking-tight sm:text-4xl">
            How it works
          </h2>
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

      <section className="border-t">
        <div className="mx-auto max-w-[82rem] px-4 py-16 sm:px-6 sm:py-24">
          <div className="flex flex-wrap items-end justify-between gap-x-8 gap-y-4">
            <h2 className="max-w-2xl font-display font-wide text-3xl leading-[1.05] font-bold tracking-tight sm:text-4xl">
              What every plan promises
            </h2>
            <Button variant="outline" asChild>
              <Link to={ROUTES.pricing}>See pricing</Link>
            </Button>
          </div>
          <div className="mt-10">
            <PromiseList row />
          </div>
        </div>
      </section>

      <section className="border-t bg-card">
        <div className="mx-auto flex max-w-[82rem] flex-wrap items-center justify-between gap-x-8 gap-y-5 px-4 py-16 sm:px-6 sm:py-24">
          <h2 className="max-w-2xl font-display font-wide text-3xl leading-[1.05] font-bold tracking-tight sm:text-4xl">
            Open your week in a few minutes.
          </h2>
          <Button size="lg" asChild>
            <Link to={ROUTES.register}>Create your account</Link>
          </Button>
        </div>
      </section>
    </PublicLayout>
  );
}
