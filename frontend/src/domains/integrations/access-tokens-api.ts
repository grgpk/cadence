import { queryOptions } from "@tanstack/react-query";

import type { AccessToken } from "../../bindings/AccessToken";
import type { CreateAccessTokenRequest } from "../../bindings/CreateAccessTokenRequest";
import type { CreatedAccessToken } from "../../bindings/CreatedAccessToken";
import { api } from "../../lib/api";
import { QUERY_KEYS } from "../../lib/query-keys";

const ACCESS_TOKENS_PATH = "/api/host/access-tokens";

export const accessTokensQuery = queryOptions({
  queryKey: QUERY_KEYS.accessTokens,
  queryFn: () => api<AccessToken[]>(ACCESS_TOKENS_PATH),
});

export function createAccessToken(
  request: CreateAccessTokenRequest,
): Promise<CreatedAccessToken> {
  return api<CreatedAccessToken>(ACCESS_TOKENS_PATH, {
    method: "POST",
    body: JSON.stringify(request),
  });
}

export function revokeAccessToken(unid: string): Promise<void> {
  return api<void>(`${ACCESS_TOKENS_PATH}/${unid}`, { method: "DELETE" });
}
