import { describe, expect, it } from "vitest";

import { forDarkPage, INK, isHexColor, readableOn } from "./color";

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

describe("forDarkPage", () => {
  it("keeps a colour that already stands out on a dark page", () => {
    expect(forDarkPage("#7c3aed")).toBe("#7c3aed");
  });

  it("lightens a dark colour until it stands out", () => {
    const shown = forDarkPage(INK);
    expect(shown).not.toBe(INK);
    expect(isHexColor(shown)).toBe(true);
    // Lightened only as far as needed: a mid grey, not white.
    expect(shown).not.toBe("#ffffff");
  });
});
