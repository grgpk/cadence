import { describe, expect, it } from "vitest";

import type { WebhookDelivery } from "../../bindings/WebhookDelivery";
import { deliveryState, maskSecret } from "./webhooks";

function delivery(overrides: Partial<WebhookDelivery>): WebhookDelivery {
  return {
    unid: "d1",
    event: "booking.created",
    attempts: 0,
    created_at: "2026-10-16T10:00:00Z",
    next_attempt_at: "2026-10-16T10:00:00Z",
    delivered_at: null,
    failed_at: null,
    last_status: null,
    last_error: null,
    ...overrides,
  };
}

describe("deliveryState", () => {
  it("names each stage of a delivery", () => {
    expect(deliveryState(delivery({}))).toEqual({ label: "Waiting", tone: "neutral" });
    expect(deliveryState(delivery({ attempts: 2, last_status: 500 }))).toEqual({
      label: "Retrying (2 tried)",
      tone: "caution",
    });
    expect(
      deliveryState(delivery({ attempts: 1, delivered_at: "x", last_status: 204 })),
    ).toEqual({ label: "Delivered 204", tone: "positive" });
    expect(deliveryState(delivery({ attempts: 8, failed_at: "x" }))).toEqual({
      label: "Gave up",
      tone: "negative",
    });
  });
});

describe("maskSecret", () => {
  it("keeps only the start", () => {
    const masked = maskSecret("whsec_0123456789abcdef");
    expect(masked.startsWith("whsec_0123")).toBe(true);
    expect(masked).not.toContain("abcdef");
  });
});
