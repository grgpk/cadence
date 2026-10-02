// Mirrors app/src/domain/call_bookings/components/slot_availability_checker.rs.
// `month` is 1-indexed (1 = January) to match the calendar UI, not JS Date's 0-index.

/** "Today" as a calendar date (Y/M/D) in the given IANA timezone. */
export function currentDateInTz(iana: string): {
  year: number;
  month: number;
  day: number;
} {
  const parts = new Intl.DateTimeFormat("en-CA", {
    timeZone: iana,
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
  }).formatToParts(new Date());
  const get = (type: string) => Number(parts.find((p) => p.type === type)?.value);
  return { year: get("year"), month: get("month"), day: get("day") };
}

function nextUtcDay(
  y: number,
  m: number,
  d: number,
): { year: number; month: number; day: number } {
  const dt = new Date(Date.UTC(y, m - 1, d));
  dt.setUTCDate(dt.getUTCDate() + 1);
  return { year: dt.getUTCFullYear(), month: dt.getUTCMonth() + 1, day: dt.getUTCDate() };
}

/** Core logic — takes `today` as a parameter for testability, mirrors calculate_from_date. */
function calculateAvailableDaysFromDate(
  today: { year: number; month: number; day: number },
  year: number,
  month: number,
): number[] {
  const next7Days: { year: number; month: number; day: number }[] = [];
  let check = today;
  while (next7Days.length < 7) {
    next7Days.push(check);
    check = nextUtcDay(check.year, check.month, check.day);
  }

  return next7Days.filter((d) => d.year === year && d.month === month).map((d) => d.day);
}

/** Calculate next 7 available days globally from today, then filter to the displayed month. */
export function calculateAvailableDays(
  iana: string,
  year: number,
  month: number,
): number[] {
  return calculateAvailableDaysFromDate(currentDateInTz(iana), year, month);
}

function daysInMonth(year: number, month: number): number {
  return new Date(Date.UTC(year, month, 0)).getUTCDate();
}

/** Mirrors get_next_month — safely handles year boundaries via date arithmetic. */
function getNextMonth(year: number, month: number): { year: number; month: number } {
  const lastDay = daysInMonth(year, month);
  const next = nextUtcDay(year, month, lastDay);
  return { year: next.year, month: next.month };
}

/** Mirrors next_month_has_days_from_date. */
function nextMonthHasDaysFromDate(
  today: { year: number; month: number; day: number },
  displayedYear: number,
  displayedMonth: number,
): boolean {
  const { year: nextYear, month: nextMonth } = getNextMonth(
    displayedYear,
    displayedMonth,
  );
  return calculateAvailableDaysFromDate(today, nextYear, nextMonth).length > 0;
}

/** Check if the next month (relative to displayed month) has any available days. */
export function nextMonthHasDays(
  iana: string,
  displayedYear: number,
  displayedMonth: number,
): boolean {
  return nextMonthHasDaysFromDate(currentDateInTz(iana), displayedYear, displayedMonth);
}
