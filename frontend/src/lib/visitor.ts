import { createServerFn } from "@tanstack/react-start";
import { getRequest } from "@tanstack/react-start/server";

export const getVisitorCountry = createServerFn({ method: "GET" }).handler(async () => {
  const country = getRequest().headers.get("cf-ipcountry")?.trim().toUpperCase();
  return country && /^[A-Z]{2}$/.test(country) ? country : null;
});
