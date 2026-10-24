import { queryOptions } from "@tanstack/react-query";

import type { AuthorizeRequest } from "../../bindings/AuthorizeRequest";
import type { OAuthClient } from "../../bindings/OAuthClient";
import type { RedirectUrl } from "../../bindings/RedirectUrl";
import { api } from "../../lib/api";
import { QUERY_KEYS } from "../../lib/query-keys";

export function oauthClientQuery(clientId: string) {
  return queryOptions({
    queryKey: QUERY_KEYS.oauthClient(clientId),
    queryFn: () => api<OAuthClient>(`/api/oauth/clients/${clientId}`),
  });
}

/** The host's answer, as the address that takes them back to the app. */
export function authorize(request: AuthorizeRequest): Promise<RedirectUrl> {
  return api<RedirectUrl>("/api/oauth/authorize", {
    method: "POST",
    body: JSON.stringify(request),
  });
}

/** Search values arrive parsed: a numeric `state` comes back as a number. */
export function oauthParam(value: unknown): string | undefined {
  if (typeof value === "number" || typeof value === "boolean") return String(value);
  return typeof value === "string" && value ? value : undefined;
}

/** Where the host goes back to, as they would recognise it. */
export function redirectHost(redirectUri: string): string {
  try {
    return new URL(redirectUri).host;
  } catch {
    return redirectUri;
  }
}
