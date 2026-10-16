import { describe, expect, it } from "vitest";

import { bookingPath, isProtectedPath, ROUTES, safeReturnPath } from "./routes";

describe("isProtectedPath", () => {
  it("covers the dashboard and admin areas", () => {
    expect(isProtectedPath("/dashboard")).toBe(true);
    expect(isProtectedPath("/dashboard/settings")).toBe(true);
    expect(isProtectedPath("/admin/leads")).toBe(true);
    expect(isProtectedPath(ROUTES.dashboardBilling)).toBe(true);
  });

  it("leaves public and unknown pages open", () => {
    expect(isProtectedPath("/")).toBe(false);
    expect(isProtectedPath("/login")).toBe(false);
    expect(isProtectedPath(ROUTES.pricing)).toBe(false);
    expect(isProtectedPath(bookingPath("ana"))).toBe(false);
    expect(isProtectedPath("/dashboards")).toBe(false);
    expect(isProtectedPath("/nope")).toBe(false);
  });
});

describe("safeReturnPath", () => {
  it("keeps a protected page with its search and hash", () => {
    expect(safeReturnPath("/dashboard/settings#time")).toBe("/dashboard/settings#time");
    expect(safeReturnPath("/admin/emails?q=Overdue")).toBe("/admin/emails?q=Overdue");
  });

  it("drops addresses on another origin", () => {
    expect(safeReturnPath("https://evil.com/dashboard")).toBeUndefined();
    expect(safeReturnPath("//evil.com/dashboard")).toBeUndefined();
    expect(safeReturnPath("/\\evil.com/dashboard")).toBeUndefined();
    expect(safeReturnPath("/\t/evil.com/dashboard")).toBeUndefined();
    expect(safeReturnPath("javascript:alert(1)")).toBeUndefined();
  });

  it("drops pages that need no login and paths that climb out", () => {
    expect(safeReturnPath("/login")).toBeUndefined();
    expect(safeReturnPath("/")).toBeUndefined();
    expect(safeReturnPath("/dashboard/../login")).toBeUndefined();
  });

  it("drops values that are not a string", () => {
    expect(safeReturnPath(undefined)).toBeUndefined();
    expect(safeReturnPath(42)).toBeUndefined();
  });
});
