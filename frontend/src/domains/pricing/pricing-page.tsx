import { Link } from "@tanstack/react-router";
import { useState } from "react";

import type { BillingInterval } from "../../bindings/BillingInterval";
import type { PlanOffer } from "../../bindings/PlanOffer";
import { Button } from "../../components/ui/button";
import { ROUTES } from "../../lib/routes";
import { PublicLayout } from "../shared/public-layout";
import { SegmentedControl } from "../shared/segmented-control";
import { CompareTable } from "./compare-table";
import { PlanCard } from "./plan-card";
import { bestYearlySaving, FEATURED_PLAN, INTERVAL, PLAN, PLAN_CTA } from "./pricing";
import { PricingFaq } from "./pricing-faq";

export function PricingPage({ offers }: { offers: readonly PlanOffer[] }) {
  const [interval, setInterval] = useState<BillingInterval>(INTERVAL.YEAR);
  const saving = bestYearlySaving(offers);

  return (
    <PublicLayout>
      <section className="pt-12 pb-16 sm:pt-20 sm:pb-24">
        <div className="mx-auto max-w-[82rem] px-4 sm:px-6">
          <h1 className="max-w-3xl motion-safe:animate-rise font-display font-wide text-5xl leading-[0.98] font-bold tracking-tight sm:text-6xl">
            Pick the plan that fits your week.
          </h1>
          <div className="mt-7 flex flex-wrap items-center justify-between gap-x-8 gap-y-5 motion-safe:animate-rise motion-safe:[animation-delay:150ms]">
            <p className="max-w-md text-lg text-muted-foreground">
              Start with 14 days of Pro, no card needed. Stay on Free as long as you like.
            </p>
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
          <div className="mt-10 grid gap-5 md:grid-cols-3">
            {offers.map((offer) => (
              <PlanCard
                key={offer.plan}
                offer={offer}
                interval={interval}
                featured={offer.plan === FEATURED_PLAN}
                action={
                  <Button
                    variant={offer.plan === FEATURED_PLAN ? "default" : "outline"}
                    asChild
                  >
                    <Link
                      to={ROUTES.register}
                      // A paid pick is carried through sign-up to checkout.
                      search={
                        offer.plan === PLAN.FREE ? {} : { plan: offer.plan, interval }
                      }
                    >
                      {PLAN_CTA[offer.plan]}
                    </Link>
                  </Button>
                }
              />
            ))}
          </div>
        </div>
      </section>

      <section className="border-t bg-card">
        <div className="mx-auto max-w-[82rem] px-4 py-16 sm:px-6 sm:py-24">
          <h2 className="font-display font-wide text-3xl font-bold tracking-tight sm:text-4xl">
            Compare plans
          </h2>
          <div className="mt-10">
            <CompareTable offers={offers} />
          </div>
        </div>
      </section>

      <section className="border-t">
        <div className="mx-auto max-w-[82rem] px-4 py-16 sm:px-6 sm:py-24">
          <h2 className="font-display font-wide text-3xl font-bold tracking-tight sm:text-4xl">
            Questions
          </h2>
          <div className="mt-8">
            <PricingFaq />
          </div>
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
