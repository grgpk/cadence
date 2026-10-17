import { describe, expect, it } from "vitest";

import { greeting } from "./greeting";

describe("greeting", () => {
  it("follows the host's clock, not the browser's", () => {
    const instant = new Date("2026-10-17T10:00:00Z");
    expect(greeting("Ana", instant, "Europe/London").text).toBe("Good morning, Ana");
    expect(greeting("Ana", instant, "Asia/Tbilisi").text).toBe("Good afternoon, Ana");
    expect(greeting("Ana", instant, "Asia/Tokyo").text).toBe("Good evening, Ana");
  });

  it("changes at noon and at five", () => {
    const at = (time: string) =>
      greeting(undefined, new Date(`2026-10-17T${time}:00Z`), "UTC");
    expect(at("11:59").phase).toBe("Morning");
    expect(at("12:00").phase).toBe("Afternoon");
    expect(at("16:59").phase).toBe("Afternoon");
    expect(at("17:00").phase).toBe("Evening");
    expect(at("23:59").text).toBe("Good evening");
  });
});
