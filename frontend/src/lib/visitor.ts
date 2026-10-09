import { createServerFn } from "@tanstack/react-start";
import { getRequest } from "@tanstack/react-start/server";

export const getVisitorCountry = createServerFn({ method: "GET" }).handler(async () => {
  const country = getRequest().headers.get("cf-ipcountry")?.trim().toUpperCase();
  return country && /^[A-Z]{2}$/.test(country) ? country : null;
});

/**
 * The address this site is being visited at, such as "https://cadence.app".
 * Link previews need the full address of their image, and only the request
 * knows it. `null` when the request's host is not a plain hostname.
 */
export const getSiteOrigin = createServerFn({ method: "GET" }).handler(async () => {
  const request = getRequest();
  const host = request.headers.get("x-forwarded-host") ?? request.headers.get("host");
  if (!host || !/^[a-z0-9.-]+(:\d+)?$/i.test(host)) return null;
  const forwarded = request.headers.get("x-forwarded-proto");
  const protocol = forwarded === "http" || forwarded === "https" ? forwarded : "http";
  return `${protocol}://${host}`;
});
