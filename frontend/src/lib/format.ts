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

/** "14 Oct 2026, 09:30" */
export function formatDateTime(value: string, timeZone?: string): string {
  return new Intl.DateTimeFormat(LOCALE, {
    dateStyle: "medium",
    timeStyle: "short",
    timeZone,
  }).format(new Date(value));
}

/** "Tue, 14 Oct" */
export function formatDay(value: string, timeZone?: string): string {
  return new Intl.DateTimeFormat(LOCALE, {
    weekday: "short",
    day: "numeric",
    month: "short",
    timeZone,
  }).format(new Date(value));
}

/** "09:30" */
export function formatTime(value: string, timeZone?: string): string {
  return new Intl.DateTimeFormat(LOCALE, {
    hour: "2-digit",
    minute: "2-digit",
    hourCycle: "h23",
    timeZone,
  }).format(new Date(value));
}

/** Turns a wall-clock "HH:MM" or "HH:MM:SS" into "9:30 AM" when the viewer wants 12h. */
export function formatClock(time: string, hour12: boolean): string {
  const [hours, minutes] = time.split(":").map(Number);
  if (!hour12)
    return `${String(hours).padStart(2, "0")}:${String(minutes).padStart(2, "0")}`;
  const period = hours < 12 ? "AM" : "PM";
  return `${hours % 12 || 12}:${String(minutes).padStart(2, "0")} ${period}`;
}
