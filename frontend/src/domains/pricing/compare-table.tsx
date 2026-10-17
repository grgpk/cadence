import { cn } from "cn";
import { Check, Minus } from "lucide-react";
import { Fragment, type ReactNode } from "react";

import type { BillingInterval } from "../../bindings/BillingInterval";
import type { Plan } from "../../bindings/Plan";
import type { PlanOffer } from "../../bindings/PlanOffer";
import { Badge } from "../../components/ui/badge";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "../../components/ui/table";
import {
  FEATURE_GROUPS,
  type FeatureValue,
  formatCents,
  PLAN_NAMES,
  priceFor,
} from "./pricing";

function Cell({ value }: { value: FeatureValue }) {
  if (value === true) {
    return (
      <>
        <Check className="mx-auto size-4 text-success" aria-hidden />
        <span className="sr-only">Included</span>
      </>
    );
  }
  if (value === false) {
    return (
      <>
        <Minus className="mx-auto size-4 text-muted-foreground/60" aria-hidden />
        <span className="sr-only">Not included</span>
      </>
    );
  }
  return <span className="font-medium">{value}</span>;
}

/** Every feature against every plan, in groups, with a way to start each plan. */
export function CompareTable({
  offers,
  interval,
  highlight,
  renderAction,
}: {
  offers: readonly PlanOffer[];
  interval: BillingInterval;
  /** The plan that fits the visitor's week: its column is tinted. */
  highlight?: Plan;
  renderAction: (offer: PlanOffer) => ReactNode;
}) {
  const tint = (plan: Plan) => plan === highlight && "bg-dusk/5";
  return (
    <Table className="min-w-[36rem] table-fixed" containerClassName="md:overflow-visible">
      {/* Pinned below the 3.5rem site header, only where the table does not
          scroll sideways: a scrolling box would pin it inside itself. */}
      <TableHeader className="z-10 bg-card md:sticky md:top-14 shadow-[inset_0_-1px_var(--color-border)]">
        <TableRow className="hover:bg-transparent">
          <TableHead className="w-2/5">
            <span className="sr-only">Feature</span>
          </TableHead>
          {offers.map((offer) => (
            <TableHead
              key={offer.plan}
              className={cn("h-auto py-3 text-center", tint(offer.plan))}
            >
              <span className="block font-semibold">{PLAN_NAMES[offer.plan]}</span>
              <span className="text-clock block text-sm font-normal text-muted-foreground">
                {formatCents(priceFor(offer, interval)?.monthly_cents ?? 0)} / month
              </span>
            </TableHead>
          ))}
        </TableRow>
      </TableHeader>
      <TableBody>
        {FEATURE_GROUPS.map((group) => (
          <Fragment key={group.title}>
            <TableRow className="bg-muted/40 hover:bg-muted/40">
              <TableHead
                scope="colgroup"
                colSpan={offers.length + 1}
                className="font-semibold"
              >
                {group.title}
              </TableHead>
            </TableRow>
            {group.rows.map((row) => (
              <TableRow key={row.label}>
                <TableHead scope="row" className="font-normal whitespace-normal">
                  <span className="flex flex-wrap items-center gap-2">
                    {row.label}
                    {row.soon ? <Badge variant="outline">Soon</Badge> : null}
                  </span>
                </TableHead>
                {offers.map((offer) => (
                  <TableCell
                    key={offer.plan}
                    className={cn("text-center", tint(offer.plan))}
                  >
                    <Cell value={row.value(offer.entitlements)} />
                  </TableCell>
                ))}
              </TableRow>
            ))}
          </Fragment>
        ))}
        <TableRow className="hover:bg-transparent">
          <TableCell />
          {offers.map((offer) => (
            <TableCell key={offer.plan} className={cn("py-4", tint(offer.plan))}>
              <div className="grid">{renderAction(offer)}</div>
            </TableCell>
          ))}
        </TableRow>
      </TableBody>
    </Table>
  );
}
