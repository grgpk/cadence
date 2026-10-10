const LOCALE = "en";

const WEEKDAY_NAMES = [
  "Monday",
  "Tuesday",
  "Wednesday",
  "Thursday",
  "Friday",
  "Saturday",
  "Sunday",
] as const;

/** Weekday name for the API's index, where 0 is Monday. */
export function weekdayName(index: number): string {
  return WEEKDAY_NAMES[index] ?? `Day ${index}`;
}

/** Whether this device writes times as "5:00 PM". The default for every time shown. */
export function prefersHour12(): boolean {
  return (
    new Intl.DateTimeFormat(undefined, { hour: "numeric" }).resolvedOptions().hour12 ??
    false
  );
}

const FORMATTERS = new Map<string, Intl.DateTimeFormat>();

/**
 * One formatter per locale and set of options. Building one is slow, and a
 * list of times asks for the same one for every row.
 */
export function dateFormatter(
  locale: string,
  options: Intl.DateTimeFormatOptions,
): Intl.DateTimeFormat {
  const key = `${locale}|${JSON.stringify(options)}`;
  let formatter = FORMATTERS.get(key);
  if (!formatter) {
    formatter = new Intl.DateTimeFormat(locale, options);
    FORMATTERS.set(key, formatter);
  }
  return formatter;
}

/** "Oct 14, 2026, 9:30 AM", or "Oct 14, 2026, 09:30" on a 24 hour device. */
export function formatDateTime(
  value: string,
  timeZone?: string,
  hour12 = prefersHour12(),
): string {
  return dateFormatter(LOCALE, {
    dateStyle: "medium",
    timeStyle: "short",
    hourCycle: hour12 ? "h12" : "h23",
    timeZone,
  }).format(new Date(value));
}

/** "Wed, Oct 14" */
export function formatDay(value: string, timeZone?: string): string {
  return dateFormatter(LOCALE, {
    weekday: "short",
    day: "numeric",
    month: "short",
    timeZone,
  }).format(new Date(value));
}

/** "Wednesday, October 14, 2026", or without the year: "Wednesday, October 14". */
export function formatLongDay(
  value: string | Date,
  timeZone?: string,
  year = true,
): string {
  return dateFormatter(LOCALE, {
    weekday: "long",
    day: "numeric",
    month: "long",
    year: year ? "numeric" : undefined,
    timeZone,
  }).format(new Date(value));
}

/** "9:30 AM", or "09:30" on a 24 hour device. */
export function formatTime(
  value: string,
  timeZone?: string,
  hour12 = prefersHour12(),
): string {
  return dateFormatter(LOCALE, {
    hour: hour12 ? "numeric" : "2-digit",
    minute: "2-digit",
    hourCycle: hour12 ? "h12" : "h23",
    timeZone,
  }).format(new Date(value));
}

/** Minutes since midnight for a wall-clock "HH:MM" or "HH:MM:SS". */
export function clockMinutes(time: string): number {
  const [hours = 0, minutes = 0] = time.split(":").map(Number);
  return hours * 60 + minutes;
}

/** The wall-clock "HH:MM" that many minutes after midnight. */
export function clockTime(minutes: number): string {
  return `${String(Math.floor(minutes / 60)).padStart(2, "0")}:${String(minutes % 60).padStart(2, "0")}`;
}

/** Turns a wall-clock "HH:MM" or "HH:MM:SS" into "9:30 AM" when the viewer wants 12h. */
export function formatClock(time: string, hour12: boolean): string {
  if (!hour12) return clockTime(clockMinutes(time));
  const total = clockMinutes(time);
  const hours = Math.floor(total / 60);
  const period = hours < 12 ? "AM" : "PM";
  return `${hours % 12 || 12}:${String(total % 60).padStart(2, "0")} ${period}`;
}

const DAY_HOURS = 24;

/** An hour on a ruler: "14:00", or "2 PM" for someone on a 12 hour clock. */
export function hourLabel(hour: number, hour12: boolean): string {
  const inDay = ((hour % DAY_HOURS) + DAY_HOURS) % DAY_HOURS;
  if (!hour12) return `${String(inDay).padStart(2, "0")}:00`;
  return `${inDay % 12 || 12} ${inDay < 12 ? "AM" : "PM"}`;
}

/** An IANA timezone name as people read it: "America/New York". */
export function zoneName(iana: string): string {
  return iana.replaceAll("_", " ");
}
