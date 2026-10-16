import { describe, expect, it } from "vitest";

import {
  DEFAULT_THEME_PREFERENCE,
  THEME_PREFERENCES,
  THEMES,
  resolveTheme,
} from "./theme";

describe("resolveTheme", () => {
  it("is dark for a device that never chose", () => {
    expect(resolveTheme(DEFAULT_THEME_PREFERENCE, false)).toBe(THEMES.DARK);
  });

  it("keeps an explicit choice whatever the system says", () => {
    expect(resolveTheme(THEME_PREFERENCES.LIGHT, true)).toBe(THEMES.LIGHT);
    expect(resolveTheme(THEME_PREFERENCES.DARK, false)).toBe(THEMES.DARK);
  });

  it("follows the system when asked to", () => {
    expect(resolveTheme(THEME_PREFERENCES.SYSTEM, true)).toBe(THEMES.DARK);
    expect(resolveTheme(THEME_PREFERENCES.SYSTEM, false)).toBe(THEMES.LIGHT);
  });
});
