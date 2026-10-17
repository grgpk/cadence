import { cn } from "cn";
import { Archive, Lock, type LucideIcon, UserCheck } from "lucide-react";

import { type PricingPromise, PROMISES } from "./pricing";

const ICONS: Record<PricingPromise, LucideIcon> = {
  neverTurnedAway: UserCheck,
  priceStays: Lock,
  nothingDeleted: Archive,
};

/** What every plan promises: stacked beside the week sizer, or in a row on the home page. */
export function PromiseList({ row = false }: { row?: boolean }) {
  return (
    <ul className={cn("grid gap-6", row && "md:grid-cols-3 md:gap-10")}>
      {(Object.keys(PROMISES) as PricingPromise[]).map((key) => {
        const Icon = ICONS[key];
        return (
          <li key={key} className="flex gap-3">
            <Icon className="mt-0.5 size-5 shrink-0 text-dusk" aria-hidden />
            <p>
              <span className="font-semibold">{PROMISES[key].title}</span>{" "}
              <span className="text-muted-foreground">{PROMISES[key].detail}</span>
            </p>
          </li>
        );
      })}
    </ul>
  );
}

/** The promises beside the week sizer on the pricing page. */
export function Promises() {
  return (
    <section aria-labelledby="promises-title">
      <h2 id="promises-title" className="text-lg font-semibold">
        On every plan
      </h2>
      <div className="mt-5">
        <PromiseList />
      </div>
    </section>
  );
}
