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

export function isCancelled(booking: Pick<Booking, "status">): boolean {
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

/** Whether a call's invitee matches what the host typed, by name or email. */
export function matchesSearch(booking: Booking, search: string): boolean {
  const wanted = search.trim().toLowerCase();
  return (
    wanted === "" ||
    booking.invitee_name.toLowerCase().includes(wanted) ||
    booking.invitee_email.toLowerCase().includes(wanted)
  );
}

/**
 * Calls still to come, soonest first; calls that have started, most recent
 * first; and cancelled calls, most recent first, whatever their time.
 */
export function splitBookings(
  bookings: Booking[],
  now: number,
): { upcoming: Booking[]; past: Booking[]; cancelled: Booking[] } {
  const upcoming: Booking[] = [];
  const past: Booking[] = [];
  const cancelled: Booking[] = [];
  for (const booking of bookings) {
    if (isCancelled(booking)) cancelled.push(booking);
    else if (new Date(booking.slot_start).getTime() >= now) upcoming.push(booking);
    else past.push(booking);
  }
  upcoming.sort((a, b) => a.slot_start.localeCompare(b.slot_start));
  past.sort((a, b) => b.slot_start.localeCompare(a.slot_start));
  cancelled.sort((a, b) => b.slot_start.localeCompare(a.slot_start));
  return { upcoming, past, cancelled };
}

export type WeekDay = { year: number; month: number; day: number };

/**
 * The seven days, Monday first, of the week `offset` weeks from the one `now`
 * falls in, on the clock of `timeZone`.
 */
export function weekDays(now: number, timeZone: string, offset: number): WeekDay[] {
  const today = zonedTime(new Date(now), timeZone);
  // Calendar days as UTC midnights, so adding days never meets a clock change.
  const midnight = Date.UTC(today.year, today.month - 1, today.day);
  const sinceMonday = (new Date(midnight).getUTCDay() + 6) % 7;
  return Array.from({ length: 7 }, (_, index) => {
    const date = new Date(midnight + (index - sinceMonday + offset * 7) * DAY_MS);
    return {
      year: date.getUTCFullYear(),
      month: date.getUTCMonth() + 1,
      day: date.getUTCDate(),
    };
  });
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
  // The next calendar day, which is not always 24 hours away.
  const { year, month, day } = zonedTime(new Date(now), timeZone);
  const next = new Date(Date.UTC(year, month - 1, day + 1));
  const tomorrow = `${next.getUTCFullYear()}-${next.getUTCMonth() + 1}-${next.getUTCDate()}`;
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

const HOUR_MINUTES = 60;

/**
 * A call as a stretch of one day on the clock of `timeZone`: the day it starts
 * on, and its start and end in minutes since that day's midnight.
 */
export function dayCall(
  booking: Booking,
  timeZone: string,
): { booking: Booking; day: WeekDay; from: number; to: number } {
  const { year, month, day, hour, minute } = zonedTime(
    new Date(booking.slot_start),
    timeZone,
  );
  const from = hour * HOUR_MINUTES + minute;
  return { booking, day: { year, month, day }, from, to: from + booking.minutes };
}

export function sameDay(a: WeekDay, b: WeekDay): boolean {
  return a.year === b.year && a.month === b.month && a.day === b.day;
}

/** "Mia" for "Mia Chen": what fits in a block on a ruler. */
export function firstName(fullName: string): string {
  return fullName.trim().split(/\s+/)[0] ?? "";
}
