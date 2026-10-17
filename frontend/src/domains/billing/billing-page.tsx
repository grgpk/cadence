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
import {
  bestYearlySaving,
  formatCents,
  INTERVAL,
  PLAN,
  PLAN_NAMES,
  priceFor,
} from "../pricing/pricing";
import { ErrorState } from "../shared/error-state";
import { PageHeader } from "../shared/page-header";
import { PanelCard } from "../shared/panel-card";
import { SegmentedControl } from "../shared/segmented-control";
import { StatusBadge, SUBSCRIPTION_STATUS } from "../shared/status-badge";
import {
  billingPlansQuery,
  billingQuery,
  goToCheckout,
  goToPortal,
  goToYearly,
} from "./billing-api";
import {
  bookingUsage,
  type CheckoutIntent,
  freeAllowanceLine,
  hasPortal,
  isBilled,
  planAction,
  STATUS,
  statusLine,
  trialDaysLeft,
  type YearlySwitch,
  yearlySwitch,
} from "./billing";
import { InviteCard } from "./invite-card";

/** After Checkout, Stripe's event can land a moment after the host does. */
const CONFIRM_POLL_MS = 2000;
const CONFIRM_WAIT_MS = 30_000;

const PAYMENT_ERRORS = {
  409: "You already pay for a plan. Change it in the billing portal.",
  503: "Payments are not set up yet. Try again later.",
};

function CurrentPlan({
  summary,
  offers,
}: {
  summary: BillingSummary;
  offers: readonly PlanOffer[];
}) {
  const usage = bookingUsage(summary);
  const limit = summary.entitlements.included_bookings_per_month;
  const allowance = freeAllowanceLine(summary, offers);
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
            {limit === null ? null : ` of ${limit} included`}
          </span>
        </div>
        {allowance ? <p className="text-sm text-muted-foreground">{allowance}</p> : null}
        {summary.trial_losses.length > 0 ? (
          <div className="rounded-lg border bg-muted/40 px-3 py-2.5 text-sm">
            <p className="font-medium">When the trial ends, on Free:</p>
            <ul className="mt-1 list-disc pl-5 text-muted-foreground">
              {summary.trial_losses.map((loss) => (
                <li key={loss}>{loss}</li>
              ))}
            </ul>
          </div>
        ) : null}
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
              // Past what the plan includes nothing breaks: a warning, not an error.
              className={usage >= 1 ? "h-full bg-warning" : "h-full bg-primary"}
              style={{ width: `${usage * 100}%` }}
            />
          </div>
        )}
      </div>
    </PanelCard>
  );
}

/** A monthly payer, one confirm screen from paying less. */
function PayYearly({
  offer,
  onSwitch,
  pending,
}: {
  offer: YearlySwitch;
  onSwitch: () => void;
  pending: boolean;
}) {
  return (
    <PanelCard
      title={`Pay yearly and save ${formatCents(offer.savedCents)}`}
      description={`${PLAN_NAMES[offer.plan]} yearly is ${formatCents(offer.yearCents)} a year, ${formatCents(offer.savedCents)} less than 12 months at today's price.`}
    >
      <div className="flex flex-wrap items-center justify-between gap-4">
        <p className="max-w-prose text-sm text-muted-foreground">
          Stripe shows the exact amount, with credit for what is left of this month,
          before you agree.
        </p>
        <Button onClick={onSwitch} disabled={pending}>
          {pending ? <Loader2 className="animate-spin" /> : null}
          Switch to yearly
        </Button>
      </div>
    </PanelCard>
  );
}

/** The plan picked on the pricing page, one click from Checkout. */
function FinishPick({
  summary,
  offer,
  interval,
  onCheckout,
  onDismiss,
  pending,
}: {
  summary: BillingSummary;
  offer: PlanOffer;
  interval: BillingInterval;
  onCheckout: () => void;
  onDismiss: () => void;
  pending: boolean;
}) {
  const name = PLAN_NAMES[offer.plan];
  const price = priceFor(offer, interval);
  const days = trialDaysLeft(summary, Date.now());
  return (
    <PanelCard
      title="Finish your pick"
      description={
        days === null
          ? `Subscribe to keep ${name} features on.`
          : `Your trial runs ${days} more ${days === 1 ? "day" : "days"}. Subscribing now keeps ${name} after it ends.`
      }
    >
      <div className="flex flex-wrap items-center justify-between gap-4">
        <p>
          <span className="font-display font-wide text-xl font-bold">{name}</span>
          <span className="ml-2 text-muted-foreground">
            {interval === INTERVAL.YEAR ? "yearly" : "monthly"}
            {price ? (
              <span className="text-clock">
                {" "}
                · {formatCents(price.amount_cents)} a{" "}
                {interval === INTERVAL.YEAR ? "year" : "month"}
                {interval === INTERVAL.YEAR
                  ? ` (${formatCents(price.monthly_cents)} / month)`
                  : null}
              </span>
            ) : null}
          </span>
        </p>
        <div className="flex flex-wrap gap-2">
          <Button variant="ghost" onClick={onDismiss} disabled={pending}>
            Decide later
          </Button>
          <Button onClick={onCheckout} disabled={pending}>
            {pending ? <Loader2 className="animate-spin" /> : null}
            Subscribe to {name}
          </Button>
        </div>
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
  onYearly,
  pending,
}: {
  summary: BillingSummary;
  offer: PlanOffer;
  interval: BillingInterval;
  featured: boolean;
  onCheckout: (plan: Plan, interval: BillingInterval) => void;
  onPortal: () => void;
  onYearly: () => void;
  pending: boolean;
}) {
  const name = PLAN_NAMES[offer.plan];
  switch (planAction(summary, offer.plan, interval)) {
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
    case "yearly":
      return (
        <Button onClick={onYearly} disabled={pending}>
          {pending ? <Loader2 className="animate-spin" /> : null}
          Switch to yearly
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
          Subscribe to {name}
        </Button>
      );
  }
}

export function BillingPage({
  intent,
  checkoutDone,
  onConfirmed,
  onDismissPick,
}: {
  intent: Partial<CheckoutIntent>;
  /** Back from Stripe Checkout. */
  checkoutDone: boolean;
  /** The paid plan arrived: the page can drop `?checkout=done`. */
  onConfirmed: () => void;
  /** Drop the pricing page's pick from the URL. */
  onDismissPick: () => void;
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
  const yearly = useMutation({ mutationFn: goToYearly, onError: paymentFailed });
  // The browser is leaving for Stripe: keep the buttons off until it has.
  const leaving = [checkout, portal, yearly].some(
    (mutation) => mutation.isPending || mutation.isSuccess,
  );

  const summary = billing.data;
  const confirmed = checkoutDone && summary !== undefined && isBilled(summary);
  const planName = summary ? PLAN_NAMES[summary.plan] : "";
  useEffect(() => {
    if (!confirmed) return;
    toast.success(`Subscribed to ${planName}.`);
    onConfirmed();
  }, [confirmed, onConfirmed, planName]);

  const featured: Plan = intent.plan ?? summary?.plan ?? PLAN.PRO;
  const saving = bestYearlySaving(plans.data ?? []);
  const toYearly = summary ? yearlySwitch(summary, plans.data ?? []) : null;
  const pickedOffer =
    intent.plan && summary && planAction(summary, intent.plan, interval) === "checkout"
      ? plans.data?.find((offer) => offer.plan === intent.plan)
      : undefined;

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
        <>
          {pickedOffer ? (
            <FinishPick
              summary={summary}
              offer={pickedOffer}
              interval={interval}
              onCheckout={() => checkout.mutate({ plan: pickedOffer.plan, interval })}
              onDismiss={onDismissPick}
              pending={leaving}
            />
          ) : null}
          <CurrentPlan summary={summary} offers={plans.data ?? []} />
          {toYearly ? (
            <PayYearly
              offer={toYearly}
              onSwitch={() => yearly.mutate()}
              pending={leaving}
            />
          ) : null}
        </>
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
                    onYearly={() => yearly.mutate()}
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
      {summary ? <InviteCard paying={summary.status === STATUS.ACTIVE} /> : null}
    </>
  );
}
