import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Loader2 } from "lucide-react";
import { useState } from "react";
import { toast } from "sonner";

import type { AdminCryptoPayment } from "../../bindings/AdminCryptoPayment";
import { Badge } from "../../components/ui/badge";
import { Button } from "../../components/ui/button";
import { Input } from "../../components/ui/input";
import { Label } from "../../components/ui/label";
import {
  Sheet,
  SheetContent,
  SheetDescription,
  SheetHeader,
  SheetTitle,
} from "../../components/ui/sheet";
import { errorMessage } from "../../lib/error-message";
import { formatDate } from "../../lib/format";
import { QUERY_KEYS } from "../../lib/query-keys";
import { formatCents, INTERVAL, PLAN_NAMES } from "../pricing/pricing";
import { ConfirmDialog } from "../shared/confirm-dialog";
import { ErrorState } from "../shared/error-state";
import { centsFrom } from "./billing";
import { adminCryptoPaymentsQuery, refundCryptoPayment } from "./billing-api";

/** The payment statuses the admin list shows. */
const STATUS_LABELS: Record<string, string> = {
  pending: "Waiting for payment",
  processing: "Confirming",
  paid: "Paid",
  refunded: "Refunded",
  underpaid: "Came in short",
};

/** Still open at the provider: nothing to refund until it settles or expires. */
const OPEN_STATUSES: readonly string[] = ["pending", "processing"];

const REFUND_ERRORS = {
  400: "Refund at least one cent, and no more than is left of the payment.",
  409: "This payment cannot be refunded here. Reload to see it as it is now.",
};

/** The money still on one paid payment, given back in part or in full. */
function RefundForm({
  payment,
  onRefunded,
}: {
  payment: AdminCryptoPayment;
  onRefunded: () => Promise<void>;
}) {
  const left = payment.amount_cents - payment.refunded_cents;
  const [dollars, setDollars] = useState((left / 100).toFixed(2));
  const [confirming, setConfirming] = useState(false);
  const cents = centsFrom(dollars);
  const full = cents === left;
  const refund = useMutation({
    // All of what is left goes without an amount, so the server takes the time back.
    mutationFn: () =>
      refundCryptoPayment(payment.unid, full ? {} : { amount_cents: cents ?? 0 }),
    onSuccess: async (made) => {
      await onRefunded();
      toast.success(
        made.claim_url
          ? `Refunded ${formatCents(made.amount_cents)}. The host was emailed a link to claim it.`
          : `Marked ${formatCents(made.amount_cents)} as refunded. Send it from the crypto provider's dashboard.`,
      );
    },
    onError: (error) => toast.error(errorMessage(error, REFUND_ERRORS)),
  });
  const valid = cents !== null && cents <= left;
  const field = `refund-${payment.unid}`;
  return (
    <div className="flex flex-wrap items-end gap-2">
      <div className="grid gap-1.5">
        <Label htmlFor={field}>Refund, in US dollars</Label>
        <Input
          id={field}
          inputMode="decimal"
          className="w-32"
          value={dollars}
          onChange={(event) => setDollars(event.target.value)}
          aria-invalid={!valid}
        />
      </div>
      <Button
        variant="outline"
        onClick={() => setConfirming(true)}
        disabled={!valid || refund.isPending}
      >
        {refund.isPending ? <Loader2 className="animate-spin" /> : null}
        Refund
      </Button>
      <ConfirmDialog
        open={confirming}
        onOpenChange={setConfirming}
        title={`Refund ${formatCents(cents ?? 0)}?`}
        description={
          full
            ? "That is all that is left of this payment, so the time it paid for comes off the plan now. This cannot be undone."
            : "Part of the payment goes back; the time it paid for stays. This cannot be undone."
        }
        confirmLabel="Refund"
        dismissLabel="Keep the payment"
        destructive
        onConfirm={() => refund.mutate()}
      />
    </div>
  );
}

/** A short payment's money back: all that arrived, once; no time changes. */
function ShortRefund({
  payment,
  onRefunded,
}: {
  payment: AdminCryptoPayment;
  onRefunded: () => Promise<void>;
}) {
  const [confirming, setConfirming] = useState(false);
  const refund = useMutation({
    mutationFn: () => refundCryptoPayment(payment.unid, {}),
    onSuccess: async (made) => {
      await onRefunded();
      toast.success(
        made.claim_url
          ? `Refunded the ${formatCents(made.amount_cents)} that arrived. The host was emailed a link to claim it.`
          : `Marked ${formatCents(made.amount_cents)} as refunded. Send it from the crypto provider's dashboard.`,
      );
    },
    onError: (error) =>
      toast.error(
        errorMessage(error, {
          409: "Nothing can be refunded on this payment. Reload to see it as it is now.",
        }),
      ),
  });
  return (
    <div className="flex flex-wrap items-center gap-3">
      <Button
        variant="outline"
        className="w-fit"
        onClick={() => setConfirming(true)}
        disabled={refund.isPending}
      >
        {refund.isPending ? <Loader2 className="animate-spin" /> : null}
        Refund what arrived
      </Button>
      <p className="text-sm text-muted-foreground">
        In the coin it came in, at the rate it was paid.
      </p>
      <ConfirmDialog
        open={confirming}
        onOpenChange={setConfirming}
        title="Refund what arrived?"
        description="The money that came in on this short invoice goes back to the host. Nothing was added to their plan for it, so their plan stays as it is."
        confirmLabel="Refund"
        dismissLabel="Keep it"
        destructive
        onConfirm={() => refund.mutate()}
      />
    </div>
  );
}

function PaymentItem({
  payment,
  onRefunded,
}: {
  payment: AdminCryptoPayment;
  onRefunded: () => Promise<void>;
}) {
  const period = payment.interval === INTERVAL.YEAR ? "1 year" : "1 month";
  const what =
    payment.kind === "upgrade"
      ? `Moved up to ${PLAN_NAMES[payment.plan]}`
      : `${PLAN_NAMES[payment.plan]}, ${period}`;
  return (
    <li className="grid gap-3 px-4 py-4">
      <div className="flex flex-wrap items-baseline justify-between gap-2">
        <p className="font-medium">{what}</p>
        <Badge
          variant={
            payment.status === "paid"
              ? "default"
              : payment.status === "underpaid"
                ? "destructive"
                : "outline"
          }
        >
          {STATUS_LABELS[payment.status] ?? payment.status}
        </Badge>
      </div>
      <dl className="grid grid-cols-2 gap-x-4 gap-y-1 text-sm">
        <dt className="text-muted-foreground">Amount</dt>
        <dd className="text-clock">
          {formatCents(payment.amount_cents)}
          {payment.refunded_cents > 0
            ? `, ${formatCents(payment.refunded_cents)} refunded`
            : null}
        </dd>
        <dt className="text-muted-foreground">{payment.paid_at ? "Paid" : "Started"}</dt>
        <dd>{formatDate(payment.paid_at ?? payment.created_at)}</dd>
        {payment.period_end && payment.status === "paid" ? (
          <>
            <dt className="text-muted-foreground">Plan until</dt>
            <dd>{formatDate(payment.period_end)}</dd>
          </>
        ) : null}
        <dt className="text-muted-foreground">Provider</dt>
        <dd>{payment.provider}</dd>
      </dl>
      {payment.status === "paid" && payment.refunded_cents < payment.amount_cents ? (
        <RefundForm payment={payment} onRefunded={onRefunded} />
      ) : payment.status === "underpaid" && payment.refunded_cents === 0 ? (
        <ShortRefund payment={payment} onRefunded={onRefunded} />
      ) : OPEN_STATUSES.includes(payment.status) ? (
        <p className="text-sm text-muted-foreground">
          The invoice is still open. Once it is paid in full or expires short, it can be
          refunded here.
        </p>
      ) : null}
    </li>
  );
}

/** Every crypto payment of one host's, with a refund for each one still paid. */
export function CryptoPaymentsSheet({
  open,
  onOpenChange,
  userUnid,
  name,
  endpoint,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  userUnid: string;
  name: string;
  /** The admin list to read again after a refund. */
  endpoint: string;
}) {
  const queryClient = useQueryClient();
  const payments = useQuery({ ...adminCryptoPaymentsQuery(userUnid), enabled: open });
  const refreshed = async () => {
    await Promise.all([
      queryClient.invalidateQueries({
        queryKey: QUERY_KEYS.adminCryptoPayments(userUnid),
      }),
      queryClient.invalidateQueries({ queryKey: QUERY_KEYS.adminData(endpoint) }),
    ]);
  };
  return (
    <Sheet open={open} onOpenChange={onOpenChange}>
      <SheetContent className="w-full gap-0 overflow-y-auto sm:max-w-lg">
        <SheetHeader className="border-b">
          <SheetTitle className="pr-8 break-words">{name}'s crypto payments</SheetTitle>
          <SheetDescription>
            Newest first. Refund part of a payment to give money back and keep its time,
            or all of it to take the time back too.
          </SheetDescription>
        </SheetHeader>
        {payments.isError ? (
          <div className="p-4">
            <ErrorState
              title="The payments did not load"
              error={payments.error}
              onRetry={() => void payments.refetch()}
            />
          </div>
        ) : payments.data ? (
          payments.data.length === 0 ? (
            <p className="p-4 text-sm text-muted-foreground">
              This host has not paid in crypto.
            </p>
          ) : (
            <ul className="divide-y">
              {payments.data.map((payment) => (
                <PaymentItem
                  key={payment.unid}
                  payment={payment}
                  onRefunded={refreshed}
                />
              ))}
            </ul>
          )
        ) : (
          <p className="flex items-center gap-2 p-4 text-sm text-muted-foreground">
            <Loader2 className="size-4 animate-spin" />
            Loading payments
          </p>
        )}
      </SheetContent>
    </Sheet>
  );
}
