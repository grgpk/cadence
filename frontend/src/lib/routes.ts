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

export function isBookingPath(pathname: string): boolean {
  return pathname.startsWith(BOOKING_PAGE_PREFIX);
}
