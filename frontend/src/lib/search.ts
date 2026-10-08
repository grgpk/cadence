/** Reads `?q=` so a link can open a list already narrowed. */
export function searchFromUrl(search: Record<string, unknown>): { q?: string } {
  return typeof search.q === "string" && search.q ? { q: search.q } : {};
}
