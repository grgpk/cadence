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
  const midnight = todayMidnight(now, timeZone);
  return Array.from({ length: 7 }, (_, index) =>
    dayAt(midnight + (index - sinceMonday(midnight) + offset * 7) * DAY_MS),
  );
}

// Calendar days as UTC midnights, so adding days never meets a clock change.
function todayMidnight(now: number, timeZone: string): number {
  const today = zonedTime(new Date(now), timeZone);
  return Date.UTC(today.year, today.month - 1, today.day);
}

function sinceMonday(midnight: number): number {
  return (new Date(midnight).getUTCDay() + 6) % 7;
}

function dayAt(midnight: number): WeekDay {
  const date = new Date(midnight);
  return {
    year: date.getUTCFullYear(),
    month: date.getUTCMonth() + 1,
    day: date.getUTCDate(),
  };
}

/** The day `offset` days from the one `now` falls in, on the clock of `timeZone`. */
export function dayOf(now: number, timeZone: string, offset: number): WeekDay {
  return dayAt(todayMidnight(now, timeZone) + offset * DAY_MS);
}

/** Whole calendar days from `from` to `to`. */
export function daysBetween(from: WeekDay, to: WeekDay): number {
  return Math.round(
    (Date.UTC(to.year, to.month - 1, to.day) -
      Date.UTC(from.year, from.month - 1, from.day)) /
      DAY_MS,
  );
}

export type MonthDay = WeekDay & { inMonth: boolean };

const MONTH_GRID_DAYS = 42;

/**
 * The month `offset` months from the one `now` falls in, as six full weeks,
 * Monday first. Days from the months either side are marked out of it.
 */
export function monthGrid(
  now: number,
  timeZone: string,
  offset: number,
): { year: number; month: number; days: MonthDay[] } {
  const today = zonedTime(new Date(now), timeZone);
  const first = Date.UTC(today.year, today.month - 1 + offset, 1);
  const { year, month } = dayAt(first);
  const start = first - sinceMonday(first) * DAY_MS;
  const days = Array.from({ length: MONTH_GRID_DAYS }, (_, index) => {
    const day = dayAt(start + index * DAY_MS);
    return { ...day, inMonth: day.month === month };
  });
  return { year, month, days };
}

/**
 * Side-by-side columns for calls that overlap on one day: each call's column
 * and how many columns its group of overlapping calls needs.
 */
export function lanes<T extends { from: number; to: number }>(
  calls: T[],
): (T & { lane: number; lanes: number })[] {
  const sorted = [...calls].sort((a, b) => a.from - b.from || b.to - a.to);
  const placed: (T & { lane: number; lanes: number })[] = [];
  let group: (T & { lane: number; lanes: number })[] = [];
  let groupEnd = Number.NEGATIVE_INFINITY;
  const closeGroup = () => {
    const width = Math.max(0, ...group.map((call) => call.lane + 1));
    for (const call of group) call.lanes = width;
    group = [];
  };
  for (const call of sorted) {
    if (call.from >= groupEnd) {
      closeGroup();
      groupEnd = call.to;
    }
    const busy = new Set(
      group.filter((other) => other.to > call.from).map((other) => other.lane),
    );
    let lane = 0;
    while (busy.has(lane)) lane += 1;
    const entry = { ...call, lane, lanes: 1 };
    group.push(entry);
    placed.push(entry);
    groupEnd = Math.max(groupEnd, call.to);
  }
  closeGroup();
  return placed;
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

const MINUTE_MS = 60_000;

/**
 * What the overview reads off the calls, on the clock of `timeZone`: every
 * call today in time order (cancelled ones too), the call under way, the next
 * call to start today, and calls still to come on later days, soonest first.
 */
export function dayPlan(
  bookings: Booking[],
  timeZone: string,
  now: number,
): { today: Booking[]; current: Booking | null; next: Booking | null; later: Booking[] } {
  const todayKey = dayKey(new Date(now), timeZone);
  const sorted = [...bookings].sort((a, b) => a.slot_start.localeCompare(b.slot_start));
  const isToday = (booking: Booking) =>
    dayKey(new Date(booking.slot_start), timeZone) === todayKey;
  const start = (booking: Booking) => new Date(booking.slot_start).getTime();
  const live = sorted.filter((booking) => !isCancelled(booking));
  return {
    today: sorted.filter(isToday),
    current:
      live.find(
        (booking) =>
          start(booking) <= now && now < start(booking) + booking.minutes * MINUTE_MS,
      ) ?? null,
    next: live.find((booking) => start(booking) > now && isToday(booking)) ?? null,
    later: live.filter((booking) => start(booking) > now && !isToday(booking)),
  };
}
