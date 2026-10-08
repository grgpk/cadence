import { describe, expect, it } from "vitest";

import {
  digitsOnly,
  formatForCountry,
  formatPhone,
  hasRepeatedDigits,
  MIN_REPEATED_DIGITS,
  phonePlaceholder,
} from "./phone-format";

describe("formatForCountry", () => {
  it("returns the country's format", () => {
    expect(formatForCountry("France")).toEqual({ groups: [1, 2, 2, 2, 2], maxDigits: 9 });
  });

  it("falls back for an unknown country", () => {
    expect(formatForCountry("Atlantis")).toEqual({ groups: [3, 3, 4], maxDigits: 10 });
  });
});

describe("digitsOnly", () => {
  it("strips everything but digits and cuts at the limit", () => {
    expect(digitsOnly("+33 (6) 12-34-56-78-99", 9)).toBe("336123456");
  });
});

describe("formatPhone", () => {
  it("groups digits for the country", () => {
    expect(formatPhone("France", "612345678")).toBe("6 12 34 56 78");
  });

  it("formats a partial number as far as it goes", () => {
    expect(formatPhone("France", "6123")).toBe("6 12 3");
    expect(formatPhone("France", "")).toBe("");
  });

  it("splits a German mobile after its prefix", () => {
    expect(formatPhone("Germany", "1701234567")).toBe("170 1234567");
    expect(formatPhone("Germany", "15123456789")).toBe("1512 3456789");
  });
});

describe("phonePlaceholder", () => {
  it("fills the country's groups", () => {
    expect(phonePlaceholder("France")).toBe("0 12 34 56 78");
    expect(phonePlaceholder("Germany")).toBe("170 1234567");
  });
});

describe("hasRepeatedDigits", () => {
  it("flags a long run of one digit", () => {
    expect(hasRepeatedDigits("611111178", MIN_REPEATED_DIGITS)).toBe(true);
  });

  it("allows shorter runs", () => {
    expect(hasRepeatedDigits("611111788", MIN_REPEATED_DIGITS)).toBe(false);
    expect(hasRepeatedDigits("612345678", MIN_REPEATED_DIGITS)).toBe(false);
  });
});
