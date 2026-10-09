export const QUERY_KEYS = {
  adminData: (endpoint: string) => ["admin-data", endpoint] as const,
  bookings: ["bookings"] as const,
  managedBooking: (token: string) => ["managed-booking", token] as const,
  bookingDetail: (unid: string) => ["booking-detail", unid] as const,
  availability: ["availability"] as const,
  hostSettings: ["host-settings"] as const,
  bookingPage: (slug: string) => ["booking-page", slug] as const,
  publicSlots: (slug: string) => ["public-slots", slug] as const,
  calendarEvents: ["google-calendar-events"] as const,
} as const;
