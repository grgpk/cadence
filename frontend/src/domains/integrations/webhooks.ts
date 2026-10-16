import type { WebhookDelivery } from "../../bindings/WebhookDelivery";
import type { WebhookEvent } from "../../bindings/WebhookEvent";

export const EVENT_LABELS: Record<WebhookEvent, string> = {
  "booking.created": "Booked",
  "booking.rescheduled": "Moved",
  "booking.cancelled": "Cancelled",
  ping: "Test",
};

type Tone = "positive" | "caution" | "negative" | "neutral";

/** Where a delivery stands, in a few words. */
export function deliveryState(delivery: WebhookDelivery): { label: string; tone: Tone } {
  if (delivery.delivered_at) {
    return { label: `Delivered ${delivery.last_status ?? ""}`.trim(), tone: "positive" };
  }
  if (delivery.failed_at) return { label: "Gave up", tone: "negative" };
  if (delivery.attempts > 0) {
    return { label: `Retrying (${delivery.attempts} tried)`, tone: "caution" };
  }
  return { label: "Waiting", tone: "neutral" };
}

/** The secret with only its start showing, until the host asks to see it. */
export function maskSecret(secret: string): string {
  return `${secret.slice(0, 10)}${"•".repeat(12)}`;
}
