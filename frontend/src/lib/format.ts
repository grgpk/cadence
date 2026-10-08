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

/** "Oct 14, 2026, 9:30 AM", or "Oct 14, 2026, 09:30" on a 24 hour device. */
export function formatDateTime(
  value: string,
  timeZone?: string,
  hour12 = prefersHour12(),
): string {
  return new Intl.DateTimeFormat(LOCALE, {
    dateStyle: "medium",
    timeStyle: "short",
    hourCycle: hour12 ? "h12" : "h23",
    timeZone,
  }).format(new Date(value));
}

/** "Wed, Oct 14" */
export function formatDay(value: string, timeZone?: string): string {
  return new Intl.DateTimeFormat(LOCALE, {
    weekday: "short",
    day: "numeric",
    month: "short",
    timeZone,
  }).format(new Date(value));
}

/** "9:30 AM", or "09:30" on a 24 hour device. */
export function formatTime(
  value: string,
  timeZone?: string,
  hour12 = prefersHour12(),
): string {
  return new Intl.DateTimeFormat(LOCALE, {
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

/** An IANA timezone name as people read it: "America/New York". */
export function zoneName(iana: string): string {
  return iana.replaceAll("_", " ");
}
