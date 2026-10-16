export const QUERY_KEYS = {
  adminData: (endpoint: string) => ["admin-data", endpoint] as const,
  bookings: ["bookings"] as const,
  managedBooking: (token: string) => ["managed-booking", token] as const,
  bookingDetail: (unid: string) => ["booking-detail", unid] as const,
  availability: ["availability"] as const,
  availabilityOverrides: ["availability-overrides"] as const,
  hostSettings: ["host-settings"] as const,
  eventTypes: ["event-types"] as const,
  webhooks: ["webhooks"] as const,
  webhookDeliveries: (unid: string) => ["webhook-deliveries", unid] as const,
  hostPage: (host: string) => ["host-page", host] as const,
  /** Without `type`, every kind of call on the host's link. */
  bookingPage: (host: string, type?: string) =>
    (type === undefined
      ? ["booking-page", host]
      : ["booking-page", host, type]) as readonly string[],
  publicSlots: (host: string, type: string) => ["public-slots", host, type] as const,
  calendarEvents: ["google-calendar-events"] as const,
  billing: ["billing"] as const,
  billingPlans: ["billing-plans"] as const,
} as const;
