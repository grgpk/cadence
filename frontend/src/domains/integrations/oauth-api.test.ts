import { describe, expect, it } from "vitest";

import { oauthParam, redirectHost } from "./oauth-api";

describe("oauthParam", () => {
  it("keeps a value the router parsed as a number or boolean as text", () => {
    expect(oauthParam(12345)).toBe("12345");
    expect(oauthParam(true)).toBe("true");
    expect(oauthParam("xyz")).toBe("xyz");
  });

  it("drops missing and empty values", () => {
    expect(oauthParam(undefined)).toBeUndefined();
    expect(oauthParam("")).toBeUndefined();
    expect(oauthParam({ nested: 1 })).toBeUndefined();
  });
});

describe("redirectHost", () => {
  it("names the app's site the host goes back to", () => {
    expect(redirectHost("https://claude.ai/api/mcp/auth_callback")).toBe("claude.ai");
    expect(redirectHost("http://localhost:6274/callback")).toBe("localhost:6274");
  });
});
