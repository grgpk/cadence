import { describe, expect, it } from "vitest";

import { INK, isHexColor, readableOn } from "./color";

describe("readableOn", () => {
  it("writes white on dark colours and ink on light ones", () => {
    expect(readableOn("#7c3aed")).toBe("#ffffff");
    expect(readableOn("#111111")).toBe("#ffffff");
    expect(readableOn("#fde047")).toBe(INK);
    expect(readableOn("#ffffff")).toBe(INK);
  });
});

describe("isHexColor", () => {
  it("takes #rrggbb only", () => {
    expect(isHexColor("#7C3AED")).toBe(true);
    expect(isHexColor("#7c3")).toBe(false);
    expect(isHexColor("red")).toBe(false);
  });
});
