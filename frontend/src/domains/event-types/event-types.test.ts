import { describe, expect, it } from "vitest";

import type { EventType } from "../../bindings/EventType";
import { canAddType, freeSlug, inputOf, lengthText, newTypeInput } from "./event-types";

function kind(slug: string): EventType {
  return {
    ...newTypeInput([]),
    slug,
    unid: `id-${slug}`,
    bookable: true,
  };
}

describe("freeSlug", () => {
  it("numbers a link the host already uses", () => {
    expect(freeSlug("new-call", [])).toBe("new-call");
    expect(freeSlug("new-call", ["new-call"])).toBe("new-call-2");
    expect(freeSlug("new-call", ["new-call", "new-call-2"])).toBe("new-call-3");
  });
});

describe("newTypeInput", () => {
  it("never takes a link the host has", () => {
    expect(newTypeInput([kind("call"), kind("new-call")]).slug).toBe("new-call-2");
  });
});

describe("inputOf", () => {
  it("leaves out the id and the plan state", () => {
    const input = inputOf(kind("call"));
    expect(input).not.toHaveProperty("unid");
    expect(input).not.toHaveProperty("bookable");
    expect(input.slug).toBe("call");
  });
});

describe("lengthText", () => {
  it("names a length, or the hours", () => {
    expect(lengthText(30)).toBe("30 min");
    expect(lengthText(90)).toBe("1 h 30 min");
    expect(lengthText(null)).toBe("Length from your hours");
  });
});

describe("canAddType", () => {
  it("follows the plan's limit", () => {
    expect(canAddType({ max_event_types: 1 }, 1)).toBe(false);
    expect(canAddType({ max_event_types: 3 }, 1)).toBe(true);
    expect(canAddType({ max_event_types: null }, 40)).toBe(true);
  });
});
