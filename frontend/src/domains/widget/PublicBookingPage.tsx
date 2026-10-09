import { Link } from "@tanstack/react-router";
import { CalendarOff } from "lucide-react";

import type { BookingPage } from "../../bindings/BookingPage";
import { APP_NAME } from "../../lib/config";
import { ROUTES } from "../../lib/routes";
import { Logo } from "../shared/Logo";
import { ThemeToggle } from "../shared/ThemeToggle";
import { BookingCard } from "./BookingCard";

/** A host's public link. The card is the page; everything else stays out of the way. */
export function PublicBookingPage({
  slug,
  page,
  initialCountryAlpha2,
}: {
  slug: string;
  page: BookingPage;
  initialCountryAlpha2?: string | null;
}) {
  return (
    <div className="flex min-h-svh flex-col">
      <main className="mx-auto flex w-full max-w-[66rem] flex-1 flex-col justify-center px-4 py-8 sm:px-6 sm:py-12">
        {page.accepting_bookings ? (
          <BookingCard
            slug={slug}
            page={page}
            initialCountryAlpha2={initialCountryAlpha2}
          />
        ) : (
          <div className="mx-auto w-full max-w-md rounded-2xl border bg-card p-8 text-center">
            <CalendarOff className="mx-auto size-8 text-muted-foreground" />
            <h1 className="mt-4 font-display font-wide text-2xl font-bold tracking-tight">
              {page.host_name} is not taking bookings right now
            </h1>
            <p className="mt-2 text-muted-foreground">
              This page is paused. Calls already booked still stand. Try this link again
              later, or ask {page.host_name} for a time.
            </p>
          </div>
        )}
      </main>
      <footer className="mx-auto flex w-full max-w-[66rem] items-center justify-between gap-4 px-4 pb-6 text-sm text-muted-foreground sm:px-6">
        <Link to={ROUTES.home} aria-label={`${APP_NAME} home`}>
          <Logo />
        </Link>
        <nav className="flex items-center gap-4">
          <Link to={ROUTES.terms} className="hover:text-foreground">
            Terms
          </Link>
          <Link to={ROUTES.privacy} className="hover:text-foreground">
            Privacy
          </Link>
          <ThemeToggle />
        </nav>
      </footer>
    </div>
  );
}
