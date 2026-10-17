import { Link } from "@tanstack/react-router";
import { cn } from "cn";
import { CalendarOff, ChevronRight, Clock } from "lucide-react";
import type { CSSProperties, ReactNode } from "react";

import type { BookingPage } from "../../bindings/BookingPage";
import type { HostPage } from "../../bindings/HostPage";
import { APP_NAME } from "../../lib/config";
import { lengthLabel } from "../../lib/format";
import { forDarkPage, readableOn } from "../../lib/color";
import { ROUTES } from "../../lib/routes";
import { Logo } from "../shared/logo";
import { ThemeToggle } from "../shared/theme-toggle";
import { BOOKING_FRAME, BookingCard } from "./booking-card";
import type { PageRef } from "./cadence-api";
import { HostBadge } from "./host-badge";
import type { SlotLayout } from "./slot-days";

/**
 * The host's colour as the theme's own: buttons, the picked time and focus
 * rings. Light and dark pages each get a version that stands out on them,
 * with text chosen for contrast; `.host-accent` in globals.css picks one.
 */
export function accentStyle(accent: string | null): CSSProperties | undefined {
  if (!accent) return undefined;
  const dark = forDarkPage(accent);
  return {
    "--accent-light": accent,
    "--accent-light-fg": readableOn(accent),
    "--accent-dark": dark,
    "--accent-dark-fg": readableOn(dark),
  } as CSSProperties;
}

/**
 * A host's public link around its content: the page, then a quiet footer.
 * On Free the footer carries Cadence, and the host's invite: a visitor who
 * signs up from it counts as theirs.
 */
function BookingShell({
  showBranding,
  accent,
  referrer,
  children,
}: {
  showBranding: boolean;
  accent: string | null;
  /** The host's booking-page address. */
  referrer: string;
  children: ReactNode;
}) {
  return (
    <div
      className={
        accent ? "host-accent flex min-h-svh flex-col" : "flex min-h-svh flex-col"
      }
      style={accentStyle(accent)}
    >
      <main className="mx-auto flex w-full max-w-[66rem] flex-1 flex-col px-4 py-4 sm:justify-center sm:px-6 sm:py-12">
        {children}
      </main>
      <footer className="mx-auto flex w-full max-w-[66rem] items-center justify-between gap-4 px-4 pb-6 text-sm text-muted-foreground sm:px-6">
        {showBranding ? (
          <div className="flex flex-wrap items-center gap-x-4 gap-y-1">
            <Link to={ROUTES.home} aria-label={`${APP_NAME} home`}>
              <Logo />
            </Link>
            <Link
              to={ROUTES.register}
              search={{ ref: referrer }}
              className="transition-colors hover:text-foreground"
            >
              Make your own booking page
            </Link>
          </div>
        ) : (
          <span />
        )}
        <nav className="flex items-center gap-4">
          <Link to={ROUTES.terms} className="transition-colors hover:text-foreground">
            Terms
          </Link>
          <Link to={ROUTES.privacy} className="transition-colors hover:text-foreground">
            Privacy
          </Link>
          <ThemeToggle />
        </nav>
      </footer>
    </div>
  );
}

function NotTakingBookings({ hostName }: { hostName: string }) {
  return (
    <div className="mx-auto w-full max-w-md rounded-2xl border bg-card p-8 text-center">
      <CalendarOff className="mx-auto size-8 text-muted-foreground" />
      <h1 className="mt-4 font-display font-wide text-2xl font-bold tracking-tight">
        {hostName} is not taking bookings right now
      </h1>
      <p className="mt-2 text-muted-foreground">
        Calls already booked still stand. Try this link again later, or ask {hostName} for
        a time.
      </p>
    </div>
  );
}

/** One kind of call on a host's link. The card is the page; everything else stays out of the way. */
export function PublicBookingPage({
  pageRef,
  page,
  initialCountryAlpha2,
  layout,
  onLayoutChange,
}: {
  pageRef: PageRef;
  page: BookingPage;
  initialCountryAlpha2?: string | null;
  layout: SlotLayout;
  onLayoutChange: (layout: SlotLayout) => void;
}) {
  return (
    <BookingShell
      showBranding={page.show_branding}
      accent={page.accent_color}
      referrer={pageRef.host}
    >
      {page.accepting_bookings ? (
        <BookingCard
          pageRef={pageRef}
          page={page}
          initialCountryAlpha2={initialCountryAlpha2}
          layout={layout}
          onLayoutChange={onLayoutChange}
        />
      ) : (
        <NotTakingBookings hostName={page.host_name} />
      )}
    </BookingShell>
  );
}

/** A host's link with several kinds of call: the visitor picks one. */
export function HostTypesPage({ slug, host }: { slug: string; host: HostPage }) {
  const taking = host.accepting_bookings && host.types.length > 0;
  return (
    <BookingShell
      showBranding={host.show_branding}
      accent={host.accent_color}
      referrer={slug}
    >
      {taking ? (
        <div className={cn(BOOKING_FRAME, "mx-auto w-full max-w-xl")}>
          <div className="px-5 pt-5 pb-4 sm:px-6 sm:pt-6">
            <HostBadge name={host.host_name} />
            <h1 className="mt-3 font-display text-2xl leading-tight font-semibold tracking-tight">
              Pick a call
            </h1>
          </div>
          <ul className="divide-y border-t">
            {host.types.map((kind) => (
              <li key={kind.slug}>
                <Link
                  to={ROUTES.bookingType}
                  params={{ slug, type: kind.slug }}
                  className="group relative flex items-center gap-4 px-5 py-4 transition-colors outline-none hover:bg-muted/40 focus-visible:bg-muted/40 focus-visible:ring-[3px] focus-visible:ring-ring/50 focus-visible:ring-inset active:bg-muted sm:px-6"
                >
                  <span
                    aria-hidden="true"
                    className="absolute inset-y-0 left-0 w-[3px] origin-center scale-y-0 bg-primary transition-transform group-hover:scale-y-100 group-focus-visible:scale-y-100"
                  />
                  <div className="min-w-0 flex-1">
                    <h2 className="font-semibold">{kind.title}</h2>
                    {kind.description ? (
                      <p className="mt-1 text-sm text-muted-foreground">
                        {kind.description}
                      </p>
                    ) : null}
                    {kind.duration_minutes ? (
                      <p className="mt-2.5 inline-flex items-center gap-1.5 rounded-full border px-2.5 py-0.5 text-xs font-medium text-muted-foreground">
                        <Clock className="size-3.5" aria-hidden />
                        {lengthLabel(kind.duration_minutes)}
                      </p>
                    ) : null}
                  </div>
                  <ChevronRight
                    className="size-5 shrink-0 text-muted-foreground transition-transform group-hover:translate-x-0.5 group-hover:text-foreground"
                    aria-hidden
                  />
                </Link>
              </li>
            ))}
          </ul>
        </div>
      ) : (
        <NotTakingBookings hostName={host.host_name} />
      )}
    </BookingShell>
  );
}
