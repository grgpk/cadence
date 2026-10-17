import { describe, expect, it } from "vitest";

import type { BillingSummary } from "../../bindings/BillingSummary";
import type { PlanOffer } from "../../bindings/PlanOffer";
import { PLAN } from "../pricing/pricing";
import {
  bookingUsage,
  checkoutIntent,
  freeAllowanceLine,
  hasPortal,
  leaveOffer,
  planAction,
  planCounts,
  signupSearch,
  STATUS,
  statusLine,
  trialDaysLeft,
  trialEnded,
  yearlySwitch,
} from "./billing";

const NOW = Date.parse("2026-10-16T12:00:00Z");

function summary(overrides: Partial<BillingSummary> = {}): BillingSummary {
  return {
    plan: PLAN.FREE,
    status: null,
    interval: null,
    trial_ends_at: null,
    current_period_end: null,
    cancel_at_period_end: false,
    bookings_this_month: 10,
    trial_losses: [],
    paused_until: null,
    paused_before: false,
    entitlements: {
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
    },
    ...overrides,
  };
}

describe("checkoutIntent", () => {
  it("keeps a paid plan with an interval", () => {
    expect(checkoutIntent({ plan: "pro", interval: "year" })).toEqual({
      plan: "pro",
      interval: "year",
    });
  });

  it("drops Free, unknown values and half an intent", () => {
    expect(checkoutIntent({ plan: "free", interval: "month" })).toEqual({});
    expect(checkoutIntent({ plan: "gold", interval: "month" })).toEqual({});
    expect(checkoutIntent({ plan: "pro" })).toEqual({});
  });
});

describe("planAction", () => {
  it("sends a free host to checkout for a paid plan", () => {
    const free = summary();
    expect(planAction(free, PLAN.FREE, "month")).toBe("current");
    expect(planAction(free, PLAN.PRO, "month")).toBe("checkout");
  });

  it("lets a trial keep Pro by paying", () => {
    const trial = summary({ plan: PLAN.PRO, status: STATUS.TRIALING });
    expect(planAction(trial, PLAN.PRO, "month")).toBe("checkout");
    expect(planAction(trial, PLAN.FREE, "month")).toBe("none");
  });

  it("sends a paying host to the portal for any change", () => {
    const paying = summary({ plan: PLAN.PRO, status: STATUS.ACTIVE });
    expect(planAction(paying, PLAN.PRO, "month")).toBe("current");
    expect(planAction(paying, PLAN.BUSINESS, "month")).toBe("portal");
    expect(planAction(paying, PLAN.FREE, "month")).toBe("portal");
  });

  it("offers a monthly payer the switch on their own card, priced yearly", () => {
    const monthly = summary({ plan: PLAN.PRO, status: STATUS.ACTIVE, interval: "month" });
    expect(planAction(monthly, PLAN.PRO, "year")).toBe("yearly");
    expect(planAction(monthly, PLAN.PRO, "month")).toBe("current");
    expect(planAction(monthly, PLAN.BUSINESS, "year")).toBe("portal");
    const yearly = { ...monthly, interval: "year" as const };
    expect(planAction(yearly, PLAN.PRO, "year")).toBe("current");
    expect(planAction(yearly, PLAN.PRO, "month")).toBe("current");
  });

  it("lets a host whose subscription ended check out again", () => {
    const ended = summary({ status: STATUS.CANCELED });
    expect(planAction(ended, PLAN.BUSINESS, "month")).toBe("checkout");
    expect(planAction(ended, PLAN.FREE, "month")).toBe("current");
  });
});

describe("hasPortal", () => {
  it("opens for hosts Stripe knows", () => {
    expect(hasPortal(summary({ status: STATUS.ACTIVE }))).toBe(true);
    expect(hasPortal(summary({ status: STATUS.CANCELED }))).toBe(true);
    expect(hasPortal(summary({ status: STATUS.TRIALING }))).toBe(false);
    expect(hasPortal(summary({ status: STATUS.COMPED }))).toBe(false);
    expect(hasPortal(summary())).toBe(false);
  });
});

describe("trialDaysLeft", () => {
  it("rounds up and stops at the end", () => {
    const ends = (iso: string) =>
      summary({ status: STATUS.TRIALING, trial_ends_at: iso });
    expect(trialDaysLeft(ends("2026-10-25T11:00:00Z"), NOW)).toBe(9);
    expect(trialDaysLeft(ends("2026-10-16T13:00:00Z"), NOW)).toBe(1);
    expect(trialDaysLeft(ends("2026-10-16T11:00:00Z"), NOW)).toBeNull();
    expect(trialDaysLeft(summary(), NOW)).toBeNull();
  });
});

describe("bookingUsage", () => {
  it("is a share of the limit, capped, and none when unlimited", () => {
    expect(bookingUsage(summary())).toBe(0.4);
    expect(bookingUsage(summary({ bookings_this_month: 40 }))).toBe(1);
    const unlimited = summary();
    unlimited.entitlements = {
      ...unlimited.entitlements,
      included_bookings_per_month: null,
    };
    expect(bookingUsage(unlimited)).toBeNull();
  });
});

describe("statusLine", () => {
  it("names the trial's days, a renewal and a failed payment", () => {
    const trial = summary({
      plan: PLAN.PRO,
      status: STATUS.TRIALING,
      trial_ends_at: "2026-10-17T11:00:00Z",
    });
    expect(statusLine(trial, NOW)).toMatch(/^1 day left of your Pro trial/);
    const renewing = summary({
      plan: PLAN.PRO,
      status: STATUS.ACTIVE,
      interval: "year",
      current_period_end: "2027-10-16T12:00:00Z",
    });
    expect(statusLine(renewing, NOW)).toMatch(
      /^Pro renews yearly on Oct 1[5-7], 2027\.$/,
    );
    expect(statusLine({ ...renewing, cancel_at_period_end: true }, NOW)).toMatch(
      /^Pro ends on .* You move to Free then, and nothing is deleted\.$/,
    );
    expect(statusLine(summary({ plan: PLAN.PRO, status: STATUS.PAST_DUE }), NOW)).toMatch(
      /payment failed/,
    );
    expect(statusLine(summary(), NOW)).toMatch(/^Free for as long/);
  });
});

describe("planCounts", () => {
  it("counts every plan, empty ones too", () => {
    expect(planCounts([{ plan: "pro" }, { plan: "free" }, { plan: "pro" }])).toEqual({
      free: 1,
      pro: 2,
      business: 0,
    });
  });
});

describe("freeAllowanceLine", () => {
  const free = summary();
  const offers: PlanOffer[] = [
    { plan: PLAN.FREE, prices: [], entitlements: free.entitlements },
  ];
  const unlimited = (bookings: number) =>
    summary({
      plan: PLAN.PRO,
      status: STATUS.TRIALING,
      bookings_this_month: bookings,
      entitlements: { ...free.entitlements, included_bookings_per_month: null },
    });

  it("measures an unlimited month against Free's limit", () => {
    expect(freeAllowanceLine(unlimited(12), offers)).toBe("Free includes 25 a month.");
    expect(freeAllowanceLine(unlimited(31), offers)).toBe(
      "That is past the 25 a month Free includes.",
    );
  });

  it("says nothing outside a trial or when Free has no limit", () => {
    expect(freeAllowanceLine(free, offers)).toBeNull();
    expect(
      freeAllowanceLine({ ...unlimited(12), status: STATUS.ACTIVE }, offers),
    ).toBeNull();
    const roomy: PlanOffer[] = [
      {
        ...offers[0],
        entitlements: { ...free.entitlements, included_bookings_per_month: null },
      } as PlanOffer,
    ];
    expect(freeAllowanceLine(unlimited(12), roomy)).toBeNull();
  });
});

describe("yearlySwitch", () => {
  const free = summary();
  const offers: PlanOffer[] = [
    {
      plan: PLAN.PRO,
      entitlements: free.entitlements,
      prices: [
        { interval: "month", amount_cents: 1000, monthly_cents: 1000 },
        { interval: "year", amount_cents: 9600, monthly_cents: 800 },
      ],
    },
  ];
  const monthly = summary({ plan: PLAN.PRO, status: STATUS.ACTIVE, interval: "month" });

  it("offers a monthly payer the yearly price and the saving", () => {
    expect(yearlySwitch(monthly, offers)).toEqual({
      plan: PLAN.PRO,
      yearCents: 9600,
      savedCents: 2400,
    });
  });

  it("stays quiet for yearly payers, trials and a plan being cancelled", () => {
    expect(yearlySwitch({ ...monthly, interval: "year" }, offers)).toBeNull();
    expect(yearlySwitch({ ...monthly, status: STATUS.TRIALING }, offers)).toBeNull();
    expect(yearlySwitch({ ...monthly, cancel_at_period_end: true }, offers)).toBeNull();
  });
});

describe("signupSearch", () => {
  it("keeps an inviter's booking-page address next to the picked plan", () => {
    expect(signupSearch({ plan: "pro", interval: "year", ref: "ada-lovelace" })).toEqual({
      plan: "pro",
      interval: "year",
      ref: "ada-lovelace",
    });
  });

  it("drops a ref that is no booking-page address", () => {
    expect(signupSearch({ ref: "Ada Lovelace!" })).toEqual({});
    expect(signupSearch({ ref: 42 })).toEqual({});
  });
});

describe("trialEnded", () => {
  it("is a trialing row whose plan is already Free", () => {
    expect(trialEnded({ status: STATUS.TRIALING, plan: PLAN.FREE })).toBe(true);
    expect(trialEnded({ status: STATUS.TRIALING, plan: PLAN.PRO })).toBe(false);
    expect(trialEnded({ status: STATUS.ACTIVE, plan: PLAN.PRO })).toBe(false);
  });
});

describe("leaveOffer", () => {
  const free = summary();
  const offers: PlanOffer[] = [
    {
      plan: PLAN.PRO,
      entitlements: free.entitlements,
      prices: [
        { interval: "month", amount_cents: 1000, monthly_cents: 1000 },
        { interval: "year", amount_cents: 9600, monthly_cents: 800 },
      ],
    },
  ];
  const monthly = summary({ plan: PLAN.PRO, status: STATUS.ACTIVE, interval: "month" });

  it("offers yearly to a monthly payer leaving over price", () => {
    expect(leaveOffer("too_expensive", monthly, offers)).toBe("yearly");
    expect(
      leaveOffer("too_expensive", { ...monthly, interval: "year" }, offers),
    ).toBeNull();
  });

  it("offers the free pause once, to a host with few calls or on a break", () => {
    expect(leaveOffer("not_enough_bookings", monthly, offers)).toBe("pause");
    expect(leaveOffer("pausing", monthly, offers)).toBe("pause");
    expect(leaveOffer("pausing", { ...monthly, paused_before: true }, offers)).toBeNull();
  });

  it("has nothing to offer for a missing feature or a move", () => {
    expect(leaveOffer("missing_feature", monthly, offers)).toBeNull();
    expect(leaveOffer("switching", monthly, offers)).toBeNull();
    expect(leaveOffer(null, monthly, offers)).toBeNull();
  });
});
