import { Link } from "@tanstack/react-router";

import { Button } from "../../components/ui/button";
import { ROUTES } from "../../lib/routes";
import { PublicLayout } from "../shared/PublicLayout";
import { DayTape } from "./DayTape";

const STEPS = [
  {
    title: "Set your hours",
    body: "Add the weekly hours when you take calls, in your own timezone. Change them whenever your week changes.",
  },
  {
    title: "Share your link",
    body: "You get one link. Send it instead of trading emails about who is free when.",
  },
  {
    title: "Get booked",
    body: "People pick a time in their timezone. Connect Google Calendar and busy times disappear and each call gets a Meet link.",
  },
];

export function LandingPage() {
  return (
    <PublicLayout>
      <section className="pt-12 pb-14 sm:pt-20 sm:pb-20">
        <div className="mx-auto max-w-[82rem] px-4 sm:px-6">
          <h1 className="max-w-4xl motion-safe:animate-rise font-display font-wide text-5xl leading-[0.98] font-bold tracking-tight sm:text-7xl">
            Your week, open for booking.
          </h1>
          <div className="mt-7 flex flex-wrap items-center gap-x-8 gap-y-5 motion-safe:animate-rise motion-safe:[animation-delay:150ms]">
            <p className="max-w-md text-lg text-muted-foreground">
              Set the hours you take calls and share one link. People pick a time, and it
              is on your calendar.
            </p>
            <Button size="lg" asChild>
              <Link to={ROUTES.register}>Create your account</Link>
            </Button>
          </div>
        </div>
        <div className="mt-12 sm:mt-16">
          <DayTape />
        </div>
      </section>

      <section className="border-t bg-card">
        <div className="mx-auto max-w-[82rem] px-4 py-12 sm:px-6 sm:py-16">
          <h2 className="font-display text-2xl font-semibold tracking-tight">
            How it works
          </h2>
          <ol className="mt-8 grid gap-8 md:grid-cols-3">
            {STEPS.map((step, index) => (
              <li key={step.title} className="border-t pt-4">
                <span className="font-display font-narrow text-3xl font-semibold text-dusk tabular-nums">
                  {index + 1}
                </span>
                <h3 className="mt-2 font-semibold">{step.title}</h3>
                <p className="mt-1 max-w-sm text-sm text-muted-foreground">{step.body}</p>
              </li>
            ))}
          </ol>
        </div>
      </section>

      <section className="border-t">
        <div className="mx-auto flex max-w-[82rem] flex-wrap items-center justify-between gap-x-8 gap-y-5 px-4 py-12 sm:px-6 sm:py-16">
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
