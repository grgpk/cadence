import { describe, expect, it } from "vitest";

import {
  searchFromUrl,
  searchInstant,
  searchMinutes,
  searchText,
  searchTimezone,
} from "./search";

describe("searchFromUrl", () => {
  it("keeps a non-empty q", () => {
    expect(searchFromUrl({ q: "Overdue" })).toEqual({ q: "Overdue" });
  });

  it("drops an empty or non-string q", () => {
    expect(searchFromUrl({ q: "" })).toEqual({});
    expect(searchFromUrl({ q: 3 })).toEqual({});
  });
});

describe("searchText", () => {
  const slug = /^[a-z0-9]+(-[a-z0-9]+)*$/;

  it("keeps a string of the expected shape", () => {
    expect(searchText("ana-k", slug)).toBe("ana-k");
  });

  it("drops other shapes and types", () => {
    expect(searchText("Ana K", slug)).toBeUndefined();
    expect(searchText(12, slug)).toBeUndefined();
  });
});

describe("searchInstant", () => {
  it("keeps a parseable instant", () => {
    expect(searchInstant("2026-10-14T09:30:00Z")).toBe("2026-10-14T09:30:00Z");
  });

  it("drops anything else", () => {
    expect(searchInstant("soon")).toBeUndefined();
    expect(searchInstant(1760434200000)).toBeUndefined();
  });
});

describe("searchMinutes", () => {
  it("keeps whole minutes up to the cap", () => {
    expect(searchMinutes(30, 240)).toBe(30);
    expect(searchMinutes("240", 240)).toBe(240);
  });

  it("drops zero, fractions and values over the cap", () => {
    expect(searchMinutes(0, 240)).toBeUndefined();
    expect(searchMinutes(12.5, 240)).toBeUndefined();
    expect(searchMinutes(241, 240)).toBeUndefined();
    expect(searchMinutes("abc", 240)).toBeUndefined();
  });
});

describe("searchTimezone", () => {
  it("keeps a zone the runtime knows", () => {
    expect(searchTimezone("Europe/Paris")).toBe("Europe/Paris");
  });

  it("drops unknown zones and unexpected shapes", () => {
    expect(searchTimezone("Mars/Olympus")).toBeUndefined();
    expect(searchTimezone("Europe/Paris<script>")).toBeUndefined();
    expect(searchTimezone(undefined)).toBeUndefined();
  });
});
