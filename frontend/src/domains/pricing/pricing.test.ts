import { describe, expect, it } from "vitest";

import type { Entitlements } from "../../bindings/Entitlements";
import type { PlanOffer } from "../../bindings/PlanOffer";
import {
  bestYearlySaving,
  callsPerMonth,
  FEATURE_GROUPS,
  fitReason,
  formatCents,
  INTERVAL,
  needsFrom,
  PLAN,
  priceFor,
  pricingSearch,
  recommendPlan,
  searchFor,
  yearlySaving,
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

const BUSINESS: Entitlements = { ...PRO, max_event_types: null, webhooks: true };

function offer(
  plan: PlanOffer["plan"],
  month: number,
  year: number,
  entitlements: Entitlements = PRO,
): PlanOffer {
  return {
    plan,
    entitlements,
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

describe("yearlySaving", () => {
  it("compares a year paid monthly with the yearly price", () => {
    expect(yearlySaving(offer(PLAN.PRO, 1000, 9600))).toEqual({
      fullYearCents: 12000,
      savedCents: 2400,
    });
    expect(
      yearlySaving({ plan: PLAN.FREE, prices: [], entitlements: FREE }),
    ).toBeUndefined();
  });
});

describe("recommendPlan", () => {
  const offers: PlanOffer[] = [
    { plan: PLAN.FREE, prices: [], entitlements: FREE },
    offer(PLAN.PRO, 1000, 9600),
    offer(PLAN.BUSINESS, 1600, 15600, BUSINESS),
  ];

  it("picks the cheapest plan that covers the week", () => {
    expect(callsPerMonth(5)).toBe(22);
    expect(recommendPlan(offers, { callsPerWeek: 5, severalKinds: false })).toBe(
      PLAN.FREE,
    );
    expect(recommendPlan(offers, { callsPerWeek: 8, severalKinds: false })).toBe(
      PLAN.PRO,
    );
    expect(recommendPlan(offers, { callsPerWeek: 2, severalKinds: true })).toBe(
      PLAN.BUSINESS,
    );
  });

  it("reads the limits from the entitlements", () => {
    const roomy = [
      { ...offers[0], entitlements: { ...FREE, max_bookings_per_month: null } },
    ];
    expect(
      recommendPlan(roomy as PlanOffer[], { callsPerWeek: 30, severalKinds: false }),
    ).toBe(PLAN.FREE);
  });

  it("says why the plan fits", () => {
    expect(fitReason(offers, { callsPerWeek: 3, severalKinds: false })).toBe(
      "Free covers your week: about 13 of 25 bookings a month.",
    );
    expect(fitReason(offers, { callsPerWeek: 8, severalKinds: false })).toBe(
      "Pro fits: unlimited bookings. Free stops at 25 a month.",
    );
    expect(fitReason(offers, { callsPerWeek: 2, severalKinds: true })).toBe(
      "Business fits: several kinds of call on one link. Free has one.",
    );
  });
});

describe("pricingSearch", () => {
  it("keeps the answers a link can carry", () => {
    expect(pricingSearch({ interval: "month", calls: "12", kinds: "several" })).toEqual({
      interval: INTERVAL.MONTH,
      calls: 12,
      kinds: "several",
    });
  });

  it("clamps calls and drops what it does not know", () => {
    expect(pricingSearch({ calls: 0 })).toEqual({ calls: 1 });
    expect(pricingSearch({ calls: 99 })).toEqual({ calls: 30 });
    expect(pricingSearch({ interval: "week", calls: "lots", kinds: "many" })).toEqual({});
  });

  it("round-trips through the needs, leaving defaults out", () => {
    expect(needsFrom({})).toEqual({ callsPerWeek: 8, severalKinds: false });
    expect(searchFor(INTERVAL.YEAR, needsFrom({}))).toEqual({});
    const search = { interval: INTERVAL.MONTH, calls: 3, kinds: "several" as const };
    expect(searchFor(INTERVAL.MONTH, needsFrom(search))).toEqual(search);
  });
});
