import { queryOptions } from "@tanstack/react-query";

import type { AccessToken } from "../../bindings/AccessToken";
import type { CreatedAccessToken } from "../../bindings/CreatedAccessToken";
import { api } from "../../lib/api";
import { QUERY_KEYS } from "../../lib/query-keys";

const ACCESS_TOKENS_PATH = "/api/host/access-tokens";

export const accessTokensQuery = queryOptions({
  queryKey: QUERY_KEYS.accessTokens,
  queryFn: () => api<AccessToken[]>(ACCESS_TOKENS_PATH),
});

export function createAccessToken(name: string): Promise<CreatedAccessToken> {
  return api<CreatedAccessToken>(ACCESS_TOKENS_PATH, {
    method: "POST",
    body: JSON.stringify({ name }),
  });
}

export function revokeAccessToken(unid: string): Promise<void> {
  return api<void>(`${ACCESS_TOKENS_PATH}/${unid}`, { method: "DELETE" });
}
