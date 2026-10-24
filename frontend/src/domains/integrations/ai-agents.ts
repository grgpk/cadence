import type { AccessTokenScope } from "../../bindings/AccessTokenScope";

/** The name agents list the server under. */
const SERVER_NAME = "cadence";

/** What each token scope is called in the Apps page. */
export const SCOPE_LABELS: Record<AccessTokenScope, string> = {
  full: "Full access",
  read: "Read only",
};

/** Where agents connect: the MCP server under the API. */
export function mcpUrl(apiUrl: string): string {
  return `${apiUrl.replace(/\/$/, "")}/api/mcp`;
}

/** The one line that adds Cadence to Claude Code. */
export function claudeCodeCommand(url: string, token: string): string {
  return `claude mcp add --transport http ${SERVER_NAME} ${url} --header "Authorization: Bearer ${token}"`;
}

/** The `mcpServers` block most other clients read from their settings file. */
export function mcpServersJson(url: string, token: string): string {
  const config = {
    mcpServers: {
      [SERVER_NAME]: {
        type: "http",
        url,
        headers: { Authorization: `Bearer ${token}` },
      },
    },
  };
  return JSON.stringify(config, null, 2);
}
