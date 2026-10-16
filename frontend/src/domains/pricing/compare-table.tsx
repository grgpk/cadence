import { Check, Minus } from "lucide-react";
import { Fragment } from "react";

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
import { FEATURE_GROUPS, type FeatureValue, PLAN_NAMES } from "./pricing";

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

/** Every feature against every plan, in groups. */
export function CompareTable({ offers }: { offers: readonly PlanOffer[] }) {
  return (
    <Table className="min-w-[36rem] table-fixed">
      <TableHeader>
        <TableRow>
          <TableHead className="w-2/5">
            <span className="sr-only">Feature</span>
          </TableHead>
          {offers.map((offer) => (
            <TableHead key={offer.plan} className="text-center font-semibold">
              {PLAN_NAMES[offer.plan]}
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
                className="text-xs tracking-wide uppercase"
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
                  <TableCell key={offer.plan} className="text-center">
                    <Cell value={row.value(offer.entitlements)} />
                  </TableCell>
                ))}
              </TableRow>
            ))}
          </Fragment>
        ))}
      </TableBody>
    </Table>
  );
}
