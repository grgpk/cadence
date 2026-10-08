/** Every app path in one place, so links and guards never repeat a string. */
export const ROUTES = {
  home: "/",
  bookingPage: "/book/$slug",
  bookingConfirmation: "/booking-confirmation",
  login: "/login",
  register: "/register",
  terms: "/terms",
  privacy: "/privacy-policy",
  dashboard: "/dashboard",
  dashboardBookings: "/dashboard/bookings",
  dashboardAvailability: "/dashboard/availability",
  dashboardSettings: "/dashboard/settings",
  admin: "/admin",
} as const;

const BOOKING_PAGE_PREFIX = ROUTES.bookingPage.replace("$slug", "");

/** A host's public booking page. */
export function bookingPath(slug: string): string {
  return `${BOOKING_PAGE_PREFIX}${slug}`;
}

const PROTECTED_ROOTS: readonly string[] = [ROUTES.dashboard, ROUTES.admin];

/** Pages that need an account. Every other path is open to a visitor. */
export function isProtectedPath(pathname: string): boolean {
  return PROTECTED_ROOTS.some(
    (root) => pathname === root || pathname.startsWith(`${root}/`),
  );
}

const LOCAL_ORIGIN = "http://local";

/**
 * The page to return to after login, taken from the URL. Only a protected page
 * of this app is accepted, so a crafted link cannot send someone elsewhere.
 */
export function safeReturnPath(value: unknown): string | undefined {
  if (typeof value !== "string") return undefined;
  let url: URL;
  try {
    url = new URL(value, LOCAL_ORIGIN);
  } catch {
    return undefined;
  }
  if (url.origin !== LOCAL_ORIGIN || !isProtectedPath(url.pathname)) return undefined;
  return `${url.pathname}${url.search}${url.hash}`;
}
