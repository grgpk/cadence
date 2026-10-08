/** Reads `?q=` so a link can open a list already narrowed. */
export function searchFromUrl(search: Record<string, unknown>): { q?: string } {
  return typeof search.q === "string" && search.q ? { q: search.q } : {};
}

/** A search value, kept only when it is a string of the expected shape. */
export function searchText(value: unknown, pattern: RegExp): string | undefined {
  return typeof value === "string" && pattern.test(value) ? value : undefined;
}

/** A search value, kept only when it parses as a point in time. */
export function searchInstant(value: unknown): string | undefined {
  return typeof value === "string" && !Number.isNaN(Date.parse(value))
    ? value
    : undefined;
}

/** A whole number of minutes from 1 to `max`. */
export function searchMinutes(value: unknown, max: number): number | undefined {
  const minutes = Number(value);
  return Number.isInteger(minutes) && minutes > 0 && minutes <= max ? minutes : undefined;
}

/** An IANA timezone name the runtime knows. */
export function searchTimezone(value: unknown): string | undefined {
  const name = searchText(value, /^[\w+-]+(\/[\w+-]+)*$/);
  if (!name) return undefined;
  try {
    new Intl.DateTimeFormat("en", { timeZone: name });
    return name;
  } catch {
    return undefined;
  }
}
