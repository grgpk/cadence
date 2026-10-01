export const QUERY_KEYS = {
  adminData: (endpoint: string) => ["admin-data", endpoint] as const,
  bookings: ["bookings"] as const,
  availability: ["availability"] as const,
  bugs: ["bugs"] as const,
  publicSlots: (hostUnid: string) => ["public-slots", hostUnid] as const,
} as const;
