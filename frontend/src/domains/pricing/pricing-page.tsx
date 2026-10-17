import { Link } from "@tanstack/react-router";
import { Check } from "lucide-react";
import { useState } from "react";

import type { BillingInterval } from "../../bindings/BillingInterval";
import type { Plan } from "../../bindings/Plan";
import type { PlanOffer } from "../../bindings/PlanOffer";
import { Button } from "../../components/ui/button";
import { ROUTES } from "../../lib/routes";
import { PublicLayout } from "../shared/public-layout";
import { SegmentedControl } from "../shared/segmented-control";
import { CompareTable } from "./compare-table";
import { PlanCard, PlanRow } from "./plan-card";
import {
  INTERVAL,
  PLAN,
  PLAN_CTA,
  PLAN_NAMES,
  recommendPlan,
  type WeekNeeds,
  yearlyLabel,
} from "./pricing";
import { PricingFaq } from "./pricing-faq";
import { Promises } from "./promises";
import { WeekSizer } from "./week-sizer";

/** Full width at phone size, so every choice is an easy tap. */
const SEGMENTS_FILL =
  "w-full sm:w-auto [&>button]:flex-1 [&>button]:justify-center [&>button]:whitespace-nowrap";

/** Sign-up, carrying a paid pick through to checkout. */
function StartButton({
  offer,
  interval,
  recommended,
  size,
}: {
  offer: PlanOffer;
  interval: BillingInterval;
  recommended: Plan;
  size?: "sm";
}) {
  return (
    <Button
      variant={offer.plan === recommended ? "default" : "outline"}
      size={size}
      asChild
    >
      <Link
        to={ROUTES.register}
        search={offer.plan === PLAN.FREE ? {} : { plan: offer.plan, interval }}
      >
        {PLAN_CTA[offer.plan]}
      </Link>
    </Button>
  );
}

export function PricingPage({
  offers,
  interval,
  needs,
  onChange,
}: {
  offers: readonly PlanOffer[];
  interval: BillingInterval;
  needs: WeekNeeds;
  onChange: (interval: BillingInterval, needs: WeekNeeds) => void;
}) {
  // The plan the phone-width table shows: the recommended one until picked by hand.
  const [compared, setCompared] = useState<Plan | null>(null);
  const recommended = recommendPlan(offers, needs);
  const recommendedOffer = offers.find((offer) => offer.plan === recommended);

  return (
    <PublicLayout>
      <section className="pt-12 pb-16 sm:pt-20 sm:pb-24">
        <div className="mx-auto max-w-[82rem] px-4 sm:px-6">
          {/* Phones read top to bottom: intro, sizer, promises. Wide screens put
              the sizer beside both, so the headline sits next to what it asks. */}
          <div className="grid items-start gap-x-14 gap-y-10 lg:grid-cols-[minmax(0,5fr)_minmax(0,7fr)] lg:grid-rows-[auto_1fr]">
            <div className="motion-safe:animate-rise lg:col-start-1 lg:row-start-1">
              <h1 className="font-display font-wide text-5xl leading-[0.98] font-bold tracking-tight sm:text-6xl">
                Pick the plan that fits your week.
              </h1>
              <p className="mt-7 max-w-md text-lg text-muted-foreground">
                Every account starts with 14 days of Pro. No card needed.
              </p>
            </div>
            <div className="motion-safe:animate-rise motion-safe:[animation-delay:150ms] lg:col-start-2 lg:row-span-2 lg:row-start-1">
              <WeekSizer
                offers={offers}
                needs={needs}
                onChange={(next) => onChange(interval, next)}
                action={
                  recommendedOffer ? (
                    <StartButton
                      offer={recommendedOffer}
                      interval={interval}
                      recommended={recommended}
                    />
                  ) : null
                }
              />
            </div>
            <div className="motion-safe:animate-rise motion-safe:[animation-delay:300ms] lg:col-start-1 lg:row-start-2">
              <Promises />
            </div>
          </div>

          <div className="mt-12 flex flex-wrap items-center justify-between gap-4">
            <h2 className="text-lg font-semibold">Plans</h2>
            <SegmentedControl
              className={SEGMENTS_FILL}
              label="Billing period"
              value={interval}
              onChange={(next) => onChange(next, needs)}
              options={[
                { value: INTERVAL.MONTH, label: "Monthly" },
                {
                  value: INTERVAL.YEAR,
                  label: yearlyLabel(offers),
                },
              ]}
            />
          </div>
          <div className="mt-5">
            <PlanRow>
              {offers.map((offer) => (
                <PlanCard
                  key={offer.plan}
                  offer={offer}
                  interval={interval}
                  fits={offer.plan === recommended}
                  action={
                    <StartButton
                      offer={offer}
                      interval={interval}
                      recommended={recommended}
                    />
                  }
                />
              ))}
            </PlanRow>
          </div>
        </div>
      </section>

      <section className="border-t bg-card">
        <div className="mx-auto max-w-[82rem] px-4 py-16 sm:px-6 sm:py-24">
          <h2 className="font-display font-wide text-3xl font-bold tracking-tight sm:text-4xl">
            Compare plans
          </h2>
          <div className="mt-8 md:hidden">
            <SegmentedControl
              className={SEGMENTS_FILL}
              label="Plan to compare"
              value={compared ?? recommended}
              onChange={setCompared}
              options={offers.map((offer) => ({
                value: offer.plan,
                label: PLAN_NAMES[offer.plan],
                icon: Check,
              }))}
            />
          </div>
          <div className="mt-6 md:mt-10">
            <CompareTable
              offers={offers}
              interval={interval}
              highlight={recommended}
              shown={compared ?? recommended}
              renderAction={(offer) => (
                <StartButton
                  offer={offer}
                  interval={interval}
                  recommended={recommended}
                  size="sm"
                />
              )}
            />
          </div>
        </div>
      </section>

      <section className="border-t">
        <div className="mx-auto grid max-w-[82rem] items-start gap-8 px-4 py-16 sm:px-6 sm:py-24 lg:grid-cols-[minmax(0,1fr)_minmax(0,2fr)] lg:gap-14">
          <h2 className="font-display font-wide text-3xl font-bold tracking-tight sm:text-4xl lg:sticky lg:top-20">
            Questions
          </h2>
          <PricingFaq />
        </div>
      </section>

      <section className="border-t bg-card">
        <div className="mx-auto flex max-w-[82rem] flex-wrap items-center justify-between gap-x-8 gap-y-5 px-4 py-16 sm:px-6 sm:py-24">
          <h2 className="max-w-2xl font-display font-wide text-3xl leading-[1.05] font-bold tracking-tight sm:text-4xl">
            Try every Pro feature for 14 days.
          </h2>
          <Button size="lg" asChild>
            <Link to={ROUTES.register}>Create your account</Link>
          </Button>
        </div>
      </section>
    </PublicLayout>
  );
}
