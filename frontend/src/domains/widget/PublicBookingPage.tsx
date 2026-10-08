import { Link } from "@tanstack/react-router";

import { Button } from "../../components/ui/button";
import { ROUTES } from "../../lib/routes";
import { PublicLayout } from "../shared/PublicLayout";
import { BookingCard } from "./BookingCard";

const STEPS = [
  {
    title: "Set your hours",
    body: "Add the weekly hours when you take calls. Change them whenever your week changes.",
  },
  {
    title: "Share your link",
    body: "Send one link instead of trading emails about who is free when.",
  },
  {
    title: "Get booked",
    body: "People pick a time in their own timezone. The call lands on your calendar with a Google Meet link.",
  },
];

export function PublicBookingPage({
  initialCountryAlpha2,
}: {
  initialCountryAlpha2?: string | null;
}) {
  return (
    <PublicLayout>
      <section className="mx-auto grid max-w-[82rem] items-start gap-10 px-4 py-10 sm:px-6 sm:py-16 xl:grid-cols-[22rem_minmax(0,1fr)] xl:gap-12">
        <div className="max-w-xl xl:pt-6">
          <h1 className="font-display text-4xl leading-[1.05] font-semibold tracking-tight sm:text-5xl">
            Book calls without the back-and-forth.
          </h1>
          <p className="mt-5 text-lg text-muted-foreground">
            Share one link. People pick a time that works, and the call shows up on both
            calendars.
          </p>
          <div className="mt-7 flex flex-wrap items-center gap-3">
            <Button size="lg" asChild>
              <Link to={ROUTES.register}>Create your account</Link>
            </Button>
            <Button size="lg" variant="outline" className="xl:hidden" asChild>
              <a href="#booking-card">Book a call</a>
            </Button>
          </div>
        </div>
        <BookingCard initialCountryAlpha2={initialCountryAlpha2} />
      </section>

      <section className="border-t bg-card">
        <div className="mx-auto max-w-[82rem] px-4 py-12 sm:px-6 sm:py-16">
          <h2 className="font-display text-2xl font-semibold tracking-tight">
            How it works
          </h2>
          <ol className="mt-8 grid gap-8 md:grid-cols-3">
            {STEPS.map((step, index) => (
              <li key={step.title} className="border-t pt-4">
                <span className="font-display text-sm font-semibold text-signal tabular-nums">
                  {index + 1}
                </span>
                <h3 className="mt-2 font-semibold">{step.title}</h3>
                <p className="mt-1 max-w-sm text-sm text-muted-foreground">{step.body}</p>
              </li>
            ))}
          </ol>
        </div>
      </section>
    </PublicLayout>
  );
}
