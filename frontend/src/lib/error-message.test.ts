import { describe, expect, it } from "vitest";

import { ApiError } from "./api";
import { errorMessage } from "./error-message";

describe("errorMessage", () => {
  it("never shows the API's own text", () => {
    expect(errorMessage(new ApiError("full_name is invalid", 400))).not.toContain(
      "full_name",
    );
    expect(errorMessage(new ApiError("database failure", 500))).not.toContain("database");
    expect(errorMessage(new Error("Cannot read properties of undefined"))).toBe(
      "Something went wrong. Try again.",
    );
  });

  it("prefers the call site's sentence for a status", () => {
    const taken = "Someone else has this link.";
    expect(errorMessage(new ApiError("link is already taken", 409), { 409: taken })).toBe(
      taken,
    );
    expect(errorMessage(new ApiError("nope", 403), { 409: taken })).toBe(
      "You do not have access to this.",
    );
  });

  it("tells a server fault from a lost connection", () => {
    expect(errorMessage(new ApiError("bad gateway", 502))).toContain("on our side");
    expect(errorMessage(new TypeError("Failed to fetch"))).toContain(
      "could not be reached",
    );
  });

  it("has a sentence for a status it does not list and for a non-error", () => {
    expect(errorMessage(new ApiError("teapot", 418))).toBe(
      "Something went wrong. Try again.",
    );
    expect(errorMessage(null)).toBe("Something went wrong. Try again.");
  });
});
