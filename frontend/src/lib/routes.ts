/** Every app path in one place, so links and guards never repeat a string. */
export const ROUTES = {
  home: "/",
  bookACall: "/book-a-call",
  bookingConfirmation: "/booking-confirmation",
  login: "/login",
  register: "/register",
  terms: "/terms",
  privacy: "/privacy-policy",
  dashboard: "/dashboard",
  dashboardBookings: "/dashboard/bookings",
  dashboardAvailability: "/dashboard/availability",
  admin: "/admin",
} as const;
