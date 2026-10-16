import { useMutation, useQuery } from "@tanstack/react-query";
import { ExternalLink, Loader2 } from "lucide-react";
import { useEffect, useState } from "react";
import { toast } from "sonner";

import type { BillingInterval } from "../../bindings/BillingInterval";
import type { BillingSummary } from "../../bindings/BillingSummary";
import type { Plan } from "../../bindings/Plan";
import type { PlanOffer } from "../../bindings/PlanOffer";
import { Button } from "../../components/ui/button";
import { Skeleton } from "../../components/ui/skeleton";
import { errorMessage } from "../../lib/error-message";
import { PlanCard } from "../pricing/plan-card";
import { bestYearlySaving, INTERVAL, PLAN, PLAN_NAMES } from "../pricing/pricing";
import { ErrorState } from "../shared/error-state";
import { PageHeader } from "../shared/page-header";
import { PanelCard } from "../shared/panel-card";
import { SegmentedControl } from "../shared/segmented-control";
import { StatusBadge, SUBSCRIPTION_STATUS } from "../shared/status-badge";
import { billingPlansQuery, billingQuery, goToCheckout, goToPortal } from "./billing-api";
import {
  bookingUsage,
  type CheckoutIntent,
  hasPortal,
  isBilled,
  planAction,
  STATUS,
  statusLine,
} from "./billing";

/** After Checkout, Stripe's event can land a moment after the host does. */
const CONFIRM_POLL_MS = 2000;
const CONFIRM_WAIT_MS = 30_000;

const PAYMENT_ERRORS = {
  409: "You already pay for a plan. Change it in the billing portal.",
  503: "Payments are not set up yet. Try again later.",
};

function CurrentPlan({ summary }: { summary: BillingSummary }) {
  const usage = bookingUsage(summary);
  const limit = summary.entitlements.max_bookings_per_month;
  return (
    <PanelCard title="Current plan" description={statusLine(summary, Date.now())}>
      <div className="flex flex-wrap items-center gap-3">
        <span className="font-display font-wide text-2xl font-bold">
          {PLAN_NAMES[summary.plan]}
        </span>
        {summary.status ? (
          <StatusBadge value={summary.status} statuses={SUBSCRIPTION_STATUS} />
        ) : null}
      </div>
      <div className="grid gap-2">
        <div className="flex items-baseline justify-between gap-4 text-sm">
          <span className="text-muted-foreground">Bookings this month</span>
          <span className="font-medium">
            {summary.bookings_this_month}
            {limit === null ? " · unlimited" : ` of ${limit}`}
          </span>
        </div>
        {usage === null ? null : (
          <div
            role="progressbar"
            aria-label="Bookings used this month"
            aria-valuemin={0}
            aria-valuemax={limit ?? 0}
            aria-valuenow={summary.bookings_this_month}
            className="h-2 overflow-hidden rounded-full bg-muted"
          >
            <div
              className={usage >= 1 ? "h-full bg-destructive" : "h-full bg-primary"}
              style={{ width: `${usage * 100}%` }}
            />
          </div>
        )}
      </div>
    </PanelCard>
  );
}

function PlanButton({
  summary,
  offer,
  interval,
  featured,
  onCheckout,
  onPortal,
  pending,
}: {
  summary: BillingSummary;
  offer: PlanOffer;
  interval: BillingInterval;
  featured: boolean;
  onCheckout: (plan: Plan, interval: BillingInterval) => void;
  onPortal: () => void;
  pending: boolean;
}) {
  const name = PLAN_NAMES[offer.plan];
  switch (planAction(summary, offer.plan)) {
    case "current":
      return (
        <Button variant="outline" disabled>
          Your plan
        </Button>
      );
    case "portal":
      return (
        <Button variant="outline" onClick={onPortal} disabled={pending}>
          Switch in billing portal
        </Button>
      );
    case "none":
      return (
        <Button variant="outline" disabled>
          {summary.status === STATUS.TRIALING ? "Yours when the trial ends" : name}
        </Button>
      );
    case "checkout":
      return (
        <Button
          variant={featured ? "default" : "outline"}
          onClick={() => onCheckout(offer.plan, interval)}
          disabled={pending}
        >
          {pending ? <Loader2 className="animate-spin" /> : null}
          {summary.plan === offer.plan ? `Keep ${name}` : `Choose ${name}`}
        </Button>
      );
  }
}

export function BillingPage({
  intent,
  checkoutDone,
  onConfirmed,
}: {
  intent: Partial<CheckoutIntent>;
  /** Back from Stripe Checkout. */
  checkoutDone: boolean;
  /** The paid plan arrived: the page can drop `?checkout=done`. */
  onConfirmed: () => void;
}) {
  const [arrivedAt] = useState(() => Date.now());
  const billing = useQuery({
    ...billingQuery,
    refetchInterval: (query) => {
      const data = query.state.data;
      const waiting = checkoutDone && !(data && isBilled(data));
      return waiting && Date.now() - arrivedAt < CONFIRM_WAIT_MS
        ? CONFIRM_POLL_MS
        : false;
    },
  });
  const plans = useQuery(billingPlansQuery);
  const [interval, setInterval] = useState<BillingInterval>(
    intent.interval ?? INTERVAL.YEAR,
  );
  const paymentFailed = (error: unknown) =>
    toast.error(errorMessage(error, PAYMENT_ERRORS));
  const checkout = useMutation({ mutationFn: goToCheckout, onError: paymentFailed });
  const portal = useMutation({ mutationFn: goToPortal, onError: paymentFailed });
  // The browser is leaving for Stripe: keep the buttons off until it has.
  const leaving =
    checkout.isPending || checkout.isSuccess || portal.isPending || portal.isSuccess;

  const summary = billing.data;
  const confirmed = checkoutDone && summary !== undefined && isBilled(summary);
  useEffect(() => {
    if (!confirmed) return;
    toast.success("Payment received. Thank you!");
    onConfirmed();
  }, [confirmed, onConfirmed]);

  const featured: Plan = intent.plan ?? summary?.plan ?? PLAN.PRO;
  const saving = bestYearlySaving(plans.data ?? []);

  return (
    <>
      <PageHeader
        title="Billing"
        description="Your plan, this month's bookings, and payments."
        actions={
          summary && hasPortal(summary) ? (
            <Button variant="outline" onClick={() => portal.mutate()} disabled={leaving}>
              <ExternalLink />
              Manage billing
            </Button>
          ) : null
        }
      />

      {billing.isError ? (
        <ErrorState
          title="Your plan did not load"
          error={billing.error}
          onRetry={() => void billing.refetch()}
        />
      ) : summary ? (
        <CurrentPlan summary={summary} />
      ) : (
        <Skeleton className="h-44 w-full" />
      )}
      {checkoutDone && !confirmed ? (
        <p className="text-sm text-muted-foreground" role="status">
          Waiting for Stripe to confirm your payment…
        </p>
      ) : null}

      <section className="grid gap-4">
        <div className="flex flex-wrap items-center justify-between gap-3">
          <h2 className="text-lg font-semibold">Plans</h2>
          <SegmentedControl
            label="Billing period"
            value={interval}
            onChange={setInterval}
            options={[
              { value: INTERVAL.MONTH, label: "Monthly" },
              {
                value: INTERVAL.YEAR,
                label: saving > 0 ? `Yearly, save ${saving}%` : "Yearly",
              },
            ]}
          />
        </div>
        {plans.isError ? (
          <ErrorState
            title="The plans did not load"
            error={plans.error}
            onRetry={() => void plans.refetch()}
          />
        ) : plans.data && summary ? (
          <div className="grid gap-4 lg:grid-cols-3">
            {plans.data.map((offer) => (
              <PlanCard
                key={offer.plan}
                offer={offer}
                interval={interval}
                featured={offer.plan === featured}
                badge={intent.plan ? "Your pick" : "Your plan"}
                action={
                  <PlanButton
                    summary={summary}
                    offer={offer}
                    interval={interval}
                    featured={offer.plan === featured}
                    onCheckout={(plan, chosen) =>
                      checkout.mutate({ plan, interval: chosen })
                    }
                    onPortal={() => portal.mutate()}
                    pending={leaving}
                  />
                }
              />
            ))}
          </div>
        ) : (
          <Skeleton className="h-96 w-full" />
        )}
      </section>
    </>
  );
}
