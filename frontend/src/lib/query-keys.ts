export const QUERY_KEYS = {
  adminData: (endpoint: string) => ["admin-data", endpoint] as const,
  bookings: ["bookings"] as const,
  availability: ["availability"] as const,
  publicSlotsAll: ["public-slots"] as const,
  publicSlots: (date: string, timezone: string) =>
    ["public-slots", date, timezone] as const,
  calendarEvents: ["google-calendar-events"] as const,
} as const;
