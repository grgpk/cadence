import { describe, expect, it } from "vitest";

import type { Entitlements } from "../../bindings/Entitlements";
import type { PlanOffer } from "../../bindings/PlanOffer";
import {
  bestYearlySaving,
  yearlyLabel,
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
  included_bookings_per_month: 25,
  reminders: ["before_24h"],
  custom_questions: false,
  custom_accent: false,
  hide_branding: false,
  max_event_types: 1,
  webhooks: false,
  win_back: false,
  attendance: false,
  insights: false,
};

const PRO: Entitlements = {
  ...FREE,
  included_bookings_per_month: null,
  reminders: ["before_24h", "before_2h", "before_30m"],
  custom_questions: true,
  max_event_types: null,
};

const BUSINESS: Entitlements = {
  ...PRO,
  webhooks: true,
  win_back: true,
  attendance: true,
  insights: true,
};

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

  it("labels the toggle with up to when the plans save differently", () => {
    const pro = offer(PLAN.PRO, 1000, 9600);
    const business = offer(PLAN.BUSINESS, 1600, 15600);
    expect(yearlyLabel([pro, business])).toBe("Yearly, save up to 20%");
    expect(yearlyLabel([pro])).toBe("Yearly, save 20%");
    expect(yearlyLabel([])).toBe("Yearly");
  });

  it("is zero without yearly prices", () => {
    expect(bestYearlySaving([{ plan: PLAN.FREE, prices: [], entitlements: FREE }])).toBe(
      0,
    );
  });
});

describe("FEATURE_GROUPS", () => {
  it("reads limits and reminders from the entitlements", () => {
    expect(row("Bookings a month").value(FREE)).toBe("25 included");
    expect(row("Bookings a month").value(PRO)).toBe("Unlimited");
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
    expect(
      recommendPlan(offers, { callsPerWeek: 5, severalKinds: false, clientTools: false }),
    ).toBe(PLAN.FREE);
    expect(
      recommendPlan(offers, { callsPerWeek: 8, severalKinds: false, clientTools: false }),
    ).toBe(PLAN.PRO);
    expect(
      recommendPlan(offers, { callsPerWeek: 2, severalKinds: true, clientTools: false }),
    ).toBe(PLAN.PRO);
  });

  it("reads the limits from the entitlements", () => {
    const roomy = [
      { ...offers[0], entitlements: { ...FREE, included_bookings_per_month: null } },
    ];
    expect(
      recommendPlan(roomy as PlanOffer[], {
        callsPerWeek: 30,
        severalKinds: false,
        clientTools: false,
      }),
    ).toBe(PLAN.FREE);
  });

  it("sends follow-ups and show-up checks to Business", () => {
    const needs = { callsPerWeek: 2, severalKinds: false, clientTools: true };
    expect(recommendPlan(offers, needs)).toBe(PLAN.BUSINESS);
    expect(fitReason(offers, needs)).toBe(
      "Business fits: follow up with people who left, see who will show up, and get your week in numbers.",
    );
  });

  it("says why the plan fits", () => {
    expect(
      fitReason(offers, { callsPerWeek: 3, severalKinds: false, clientTools: false }),
    ).toBe("Free covers your week: about 13 of the 25 bookings it includes each month.");
    expect(
      fitReason(offers, { callsPerWeek: 8, severalKinds: false, clientTools: false }),
    ).toBe("Pro fits: about 35 bookings a month, past the 25 Free includes.");
    expect(
      fitReason(offers, { callsPerWeek: 2, severalKinds: true, clientTools: false }),
    ).toBe("Pro fits: several kinds of call on one link. Free has one.");
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
    expect(needsFrom({})).toEqual({
      callsPerWeek: 8,
      severalKinds: false,
      clientTools: false,
    });
    expect(searchFor(INTERVAL.YEAR, needsFrom({}))).toEqual({});
    const search = {
      interval: INTERVAL.MONTH,
      calls: 3,
      kinds: "several" as const,
      tools: "yes" as const,
    };
    expect(searchFor(INTERVAL.MONTH, needsFrom(search))).toEqual(search);
  });
});
