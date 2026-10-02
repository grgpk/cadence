type CalendarCell = number | null;

export function monthLabel(year: number, month: number): string {
  return new Intl.DateTimeFormat("en-US", {
    month: "long",
    year: "numeric",
  }).format(new Date(year, month - 1, 1));
}

export function calendarWeeks(
  year: number,
  month: number,
  weekStartsMonday = false,
): CalendarCell[][] {
  const sundayIndex = new Date(year, month - 1, 1).getDay();
  const firstWeekday = weekStartsMonday ? (sundayIndex + 6) % 7 : sundayIndex;
  const daysInMonth = new Date(year, month, 0).getDate();
  const cells: CalendarCell[] = [
    ...Array.from({ length: firstWeekday }, () => null),
    ...Array.from({ length: daysInMonth }, (_, index) => index + 1),
  ];
  while (cells.length % 7 !== 0) cells.push(null);
  return Array.from({ length: cells.length / 7 }, (_, index) =>
    cells.slice(index * 7, index * 7 + 7),
  );
}

export function isoDate(year: number, month: number, day: number): string {
  return `${year.toString().padStart(4, "0")}-${month.toString().padStart(2, "0")}-${day.toString().padStart(2, "0")}`;
}

export function shiftMonth(
  year: number,
  month: number,
  offset: number,
): { year: number; month: number } {
  const date = new Date(year, month - 1 + offset, 1);
  return { year: date.getFullYear(), month: date.getMonth() + 1 };
}
