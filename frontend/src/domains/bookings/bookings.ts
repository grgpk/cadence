import { queryOptions } from "@tanstack/react-query";

import type { Booking } from "../../bindings/Booking";
import type { BookingStatus } from "../../bindings/BookingStatus";
import { api } from "../../lib/api";
import { QUERY_KEYS } from "../../lib/query-keys";

const CANCELLED: BookingStatus = "cancelled";

/** The signed-in host's bookings. */
export const bookingsQuery = queryOptions({
  queryKey: QUERY_KEYS.bookings,
  queryFn: () => api<Booking[]>("/api/bookings"),
});

/**
 * Calls still to come, soonest first, and everything else, most recent first.
 * A cancelled call is never to come, whatever its time.
 */
export function splitBookings(
  bookings: Booking[],
  now: number,
): { upcoming: Booking[]; past: Booking[] } {
  const upcoming: Booking[] = [];
  const past: Booking[] = [];
  for (const booking of bookings) {
    const toCome =
      booking.status !== CANCELLED && new Date(booking.slot_start).getTime() >= now;
    (toCome ? upcoming : past).push(booking);
  }
  upcoming.sort((a, b) => a.slot_start.localeCompare(b.slot_start));
  past.sort((a, b) => b.slot_start.localeCompare(a.slot_start));
  return { upcoming, past };
}
