import { queryOptions } from "@tanstack/react-query";

import type { Booking } from "../../bindings/Booking";
import type { BookingDetail } from "../../bindings/BookingDetail";
import type { BookingStatus } from "../../bindings/BookingStatus";
import { api } from "../../lib/api";
import { formatDay } from "../../lib/format";
import { QUERY_KEYS } from "../../lib/query-keys";
import { zonedTime } from "../widget/zoned-time";

const CANCELLED: BookingStatus = "cancelled";
const DAY_MS = 24 * 60 * 60 * 1000;

export function isCancelled(booking: Booking): boolean {
  return booking.status === CANCELLED;
}

/** The signed-in host's bookings. */
export const bookingsQuery = queryOptions({
  queryKey: QUERY_KEYS.bookings,
  queryFn: () => api<Booking[]>("/api/bookings"),
});

/** One of the host's calls, with the invitee's answers. */
export const bookingDetailQuery = (unid: string) =>
  queryOptions({
    queryKey: QUERY_KEYS.bookingDetail(unid),
    queryFn: () => api<BookingDetail>(`/api/bookings/${unid}`),
  });

export function cancelBooking(unid: string): Promise<Booking> {
  return api<Booking>(`/api/bookings/${unid}/cancel`, { method: "POST" });
}

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
    const toCome = !isCancelled(booking) && new Date(booking.slot_start).getTime() >= now;
    (toCome ? upcoming : past).push(booking);
  }
  upcoming.sort((a, b) => a.slot_start.localeCompare(b.slot_start));
  past.sort((a, b) => b.slot_start.localeCompare(a.slot_start));
  return { upcoming, past };
}

type AgendaDay = { key: string; label: string; bookings: Booking[] };

function dayKey(instant: Date, timeZone: string): string {
  const { year, month, day } = zonedTime(instant, timeZone);
  return `${year}-${month}-${day}`;
}

/**
 * Calls grouped by the day they fall on in `timeZone`, in the order given:
 * "Today", "Tomorrow", then the date.
 */
export function agendaDays(
  bookings: Booking[],
  timeZone: string,
  now: number,
): AgendaDay[] {
  const today = dayKey(new Date(now), timeZone);
  const tomorrow = dayKey(new Date(now + DAY_MS), timeZone);
  const days: AgendaDay[] = [];
  for (const booking of bookings) {
    const key = dayKey(new Date(booking.slot_start), timeZone);
    const last = days.at(-1);
    if (last?.key === key) {
      last.bookings.push(booking);
      continue;
    }
    const label =
      key === today
        ? "Today"
        : key === tomorrow
          ? "Tomorrow"
          : formatDay(booking.slot_start, timeZone);
    days.push({ key, label, bookings: [booking] });
  }
  return days;
}
