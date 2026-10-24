import { describe, expect, it } from "vitest";

import { claudeCodeCommand, mcpServersJson, mcpUrl } from "./ai-agents";

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
