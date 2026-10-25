import { describe, expect, it } from "vitest";

import type { AccessToken } from "../../bindings/AccessToken";
import { claudeCodeCommand, mcpServersJson, mcpUrl, splitTokens } from "./ai-agents";

const TOKEN = "cad_0123abcd";

describe("mcpUrl", () => {
  it("puts the server under the API, with or without a trailing slash", () => {
    expect(mcpUrl("https://bookcadence.app")).toBe("https://bookcadence.app/api/mcp");
    expect(mcpUrl("http://localhost:3001/")).toBe("http://localhost:3001/api/mcp");
  });
});

describe("setup snippets", () => {
  const url = mcpUrl("https://bookcadence.app");

  it("gives Claude Code one command with the token as a header", () => {
    expect(claudeCodeCommand(url, TOKEN)).toBe(
      'claude mcp add --transport http cadence https://bookcadence.app/api/mcp --header "Authorization: Bearer cad_0123abcd"',
    );
  });

  it("gives other clients an mcpServers block", () => {
    expect(JSON.parse(mcpServersJson(url, TOKEN))).toEqual({
      mcpServers: {
        cadence: {
          type: "http",
          url,
          headers: { Authorization: `Bearer ${TOKEN}` },
        },
      },
    });
  });
});

describe("splitTokens", () => {
  const token = (name: string, connector: boolean): AccessToken => ({
    unid: name,
    name,
    last_four: "abcd",
    scope: "full",
    last_used_at: null,
    created_at: "2026-10-25T12:00:00Z",
    connector,
  });

  it("puts signed-in apps apart from hand-made tokens, keeping their order", () => {
    const tokens = [
      token("Script", false),
      token("Claude", true),
      token("Laptop", false),
    ];
    const { connected, personal } = splitTokens(tokens);
    expect(connected.map((t) => t.name)).toEqual(["Claude"]);
    expect(personal.map((t) => t.name)).toEqual(["Script", "Laptop"]);
  });
});
