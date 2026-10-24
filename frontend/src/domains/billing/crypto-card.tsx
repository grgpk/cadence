import { Bitcoin, Loader2 } from "lucide-react";
import { useState } from "react";

import type { BillingInterval } from "../../bindings/BillingInterval";
import type { CryptoSummary } from "../../bindings/CryptoSummary";
import type { Plan } from "../../bindings/Plan";
import type { PlanOffer } from "../../bindings/PlanOffer";
import type { StartCheckout } from "../../bindings/StartCheckout";
import { Button } from "../../components/ui/button";
import { formatCents, INTERVAL, PLAN, PLAN_NAMES, priceFor } from "../pricing/pricing";
import { PanelCard } from "../shared/panel-card";
import { SegmentedControl } from "../shared/segmented-control";
import { cryptoStatusLine } from "./billing";

type PaidPlan = typeof PLAN.PRO | typeof PLAN.BUSINESS;

const PLAN_OPTIONS: readonly { value: PaidPlan; label: string }[] = [
  { value: PLAN.PRO, label: PLAN_NAMES.pro },
  { value: PLAN.BUSINESS, label: PLAN_NAMES.business },
];
const INTERVAL_OPTIONS: readonly { value: BillingInterval; label: string }[] = [
  { value: INTERVAL.MONTH, label: "1 month" },
  { value: INTERVAL.YEAR, label: "1 year" },
];

/**
 * The opt-in to pay in crypto: a month or a year up front, next to Stripe's
 * plans and apart from them. A running period can only be renewed, not
 * switched to another plan.
 */
export function CryptoCard({
  crypto,
  offers,
  onCheckout,
  pending,
}: {
  crypto: CryptoSummary;
  offers: readonly PlanOffer[];
  onCheckout: (choice: StartCheckout) => void;
  pending: boolean;
}) {
  const [picked, setPicked] = useState<PaidPlan>(PLAN.PRO);
  const [interval, setInterval] = useState<BillingInterval>(INTERVAL.YEAR);
  const plan: Plan = crypto.active?.plan ?? picked;
  const offer = offers.find((candidate) => candidate.plan === plan);
  const price = offer ? priceFor(offer, interval) : undefined;
  return (
    <PanelCard
      title="Pay with crypto"
      description={
        crypto.active
          ? cryptoStatusLine(crypto.active)
          : "Pay for a month or a year up front in Bitcoin. Nothing renews by itself, and we email you 3 days before it ends."
      }
    >
      {crypto.confirming ? (
        <p
          role="status"
          className="flex items-center gap-2 rounded-lg border bg-muted/40 px-3 py-2.5 text-sm"
        >
          <Loader2 className="size-4 animate-spin" />
          Your payment is confirming on the network. This page updates when it is done.
        </p>
      ) : null}
      <div className="flex flex-wrap items-center justify-between gap-4">
        <div className="flex flex-wrap items-center gap-3">
          {crypto.active ? null : (
            <SegmentedControl
              label="Plan to pay for"
              value={picked}
              onChange={setPicked}
              options={PLAN_OPTIONS}
            />
          )}
          <SegmentedControl
            label="Period to pay for"
            value={interval}
            onChange={setInterval}
            options={INTERVAL_OPTIONS}
          />
          {price ? (
            <span className="text-clock text-sm text-muted-foreground">
              {formatCents(price.amount_cents)}
            </span>
          ) : null}
        </div>
        <Button
          variant="outline"
          onClick={() => onCheckout({ plan, interval })}
          disabled={pending}
        >
          {pending ? <Loader2 className="animate-spin" /> : <Bitcoin />}
          {crypto.active ? `Renew ${PLAN_NAMES[plan]} with crypto` : "Pay with crypto"}
        </Button>
      </div>
    </PanelCard>
  );
}
