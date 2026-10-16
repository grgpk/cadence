import { describe, expect, it } from "vitest";

import type { Entitlements } from "../../bindings/Entitlements";
import type { PlanOffer } from "../../bindings/PlanOffer";
import {
  bestYearlySaving,
  FEATURE_GROUPS,
  formatCents,
  INTERVAL,
  PLAN,
  priceFor,
} from "./pricing";

const FREE: Entitlements = {
  max_bookings_per_month: 25,
  reminders: ["before_24h"],
  custom_questions: false,
  custom_accent: false,
  hide_branding: false,
  max_event_types: 1,
  webhooks: false,
};

const PRO: Entitlements = {
  ...FREE,
  max_bookings_per_month: null,
  reminders: ["before_24h", "before_2h", "before_30m"],
  custom_questions: true,
};

function offer(plan: PlanOffer["plan"], month: number, year: number): PlanOffer {
  return {
    plan,
    entitlements: PRO,
    prices: [
      { interval: INTERVAL.MONTH, amount_cents: month, monthly_cents: month },
      { interval: INTERVAL.YEAR, amount_cents: year, monthly_cents: year / 12 },
    ],
  };
}

function row(label: string) {
  const found = FEATURE_GROUPS.flatMap((group) => group.rows).find(
    (candidate) => candidate.label === label,
  );
  if (!found) throw new Error(`no row ${label}`);
  return found;
}

describe("priceFor", () => {
  it("finds the price of an interval, and none for Free", () => {
    const pro = offer(PLAN.PRO, 1000, 9600);
    expect(priceFor(pro, INTERVAL.YEAR)?.monthly_cents).toBe(800);
    expect(
      priceFor({ ...pro, plan: PLAN.FREE, prices: [] }, INTERVAL.MONTH),
    ).toBeUndefined();
  });
});

describe("formatCents", () => {
  it("drops cents from whole dollars only", () => {
    expect(formatCents(800)).toBe("$8");
    expect(formatCents(1250)).toBe("$12.50");
    expect(formatCents(0)).toBe("$0");
  });
});

describe("bestYearlySaving", () => {
  it("rounds the best saving down", () => {
    // Pro saves exactly 20%, Business 18.75%.
    const offers = [offer(PLAN.PRO, 1000, 9600), offer(PLAN.BUSINESS, 1600, 15600)];
    expect(bestYearlySaving(offers)).toBe(20);
    expect(bestYearlySaving([offers[1] as PlanOffer])).toBe(18);
  });

  it("is zero without yearly prices", () => {
    expect(bestYearlySaving([{ plan: PLAN.FREE, prices: [], entitlements: FREE }])).toBe(
      0,
    );
  });
});

describe("FEATURE_GROUPS", () => {
  it("reads limits and reminders from the entitlements", () => {
    expect(row("Bookings per month").value(FREE)).toBe("25 a month");
    expect(row("Bookings per month").value(PRO)).toBe("Unlimited");
    expect(row("Reminders before the call").value(FREE)).toBe("24h");
    expect(row("Reminders before the call").value(PRO)).toBe("24h, 2h, 30 min");
    expect(row("Your own booking form questions").value(FREE)).toBe(false);
  });
});
