import { useMutation, useQueryClient } from "@tanstack/react-query";
import { Loader2 } from "lucide-react";
import { useId, useState } from "react";
import { toast } from "sonner";

import type { BillingSummary } from "../../bindings/BillingSummary";
import type { CancelReason } from "../../bindings/CancelReason";
import type { PlanOffer } from "../../bindings/PlanOffer";
import {
  AlertDialog,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "../../components/ui/alert-dialog";
import { Button } from "../../components/ui/button";
import { Label } from "../../components/ui/label";
import { RadioGroup, RadioGroupItem } from "../../components/ui/radio-group";
import { Textarea } from "../../components/ui/textarea";
import { errorMessage } from "../../lib/error-message";
import { formatDate } from "../../lib/format";
import { QUERY_KEYS } from "../../lib/query-keys";
import { formatCents } from "../pricing/pricing";
import {
  cancelPlan,
  goToYearly,
  keepPlan,
  pausePlan,
  yearlyInstead,
} from "./billing-api";
import { LEAVE_REASONS, leaveOffer, yearlySwitch } from "./billing";

const NOTE_MAX = 500;
const LEAVE_ERRORS = {
  409: "Your plan changed in the meantime. Reload to see it as it is now.",
  503: "Payments are not set up yet. Try again later.",
};

/** "Cancel plan", asking why first and offering what fits the answer. */
export function CancelPlan({
  summary,
  offers,
}: {
  summary: BillingSummary;
  offers: readonly PlanOffer[];
}) {
  const id = useId();
  const queryClient = useQueryClient();
  const [open, setOpen] = useState(false);
  const [reason, setReason] = useState<CancelReason | null>(null);
  const [note, setNote] = useState("");
  const feedback = () => ({
    reason: reason ?? "other",
    ...(note.trim() ? { note: note.trim() } : {}),
  });
  const done = async (message: string) => {
    await queryClient.invalidateQueries({ queryKey: QUERY_KEYS.billing });
    setOpen(false);
    toast.success(message);
  };
  const failed = (error: unknown) => toast.error(errorMessage(error, LEAVE_ERRORS));
  const ends = summary.current_period_end ? formatDate(summary.current_period_end) : null;
  const cancel = useMutation({
    mutationFn: () => cancelPlan(feedback()),
    onSuccess: () =>
      done(
        ends
          ? `Your plan ends on ${ends}. Your page stays open on Free.`
          : "Your plan ends with this period.",
      ),
    onError: failed,
  });
  const pause = useMutation({
    mutationFn: () => pausePlan(feedback()),
    onSuccess: () => done("Your next month is free. Your plan stays on."),
    onError: failed,
  });
  const yearly = useMutation({
    mutationFn: async () => {
      await yearlyInstead(feedback());
      await goToYearly();
    },
    onError: failed,
  });
  const busy = [cancel, pause, yearly].some((mutation) => mutation.isPending);
  const offer = leaveOffer(reason, summary, offers);
  const saving = yearlySwitch(summary, offers);

  return (
    <>
      <Button variant="ghost" size="sm" onClick={() => setOpen(true)}>
        Cancel plan
      </Button>
      <AlertDialog open={open} onOpenChange={setOpen}>
        <AlertDialogContent className="sm:max-w-lg">
          <AlertDialogHeader>
            <AlertDialogTitle>Before you go</AlertDialogTitle>
            <AlertDialogDescription>
              Why are you leaving? It helps us make Cadence better.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <RadioGroup
            value={reason ?? ""}
            onValueChange={(value) => setReason(value as CancelReason)}
            className="grid gap-2.5 sm:grid-cols-2"
            aria-label="Why are you leaving?"
          >
            {LEAVE_REASONS.map((option) => (
              <div key={option.value} className="flex items-center gap-2">
                <RadioGroupItem value={option.value} id={`${id}-${option.value}`} />
                <Label htmlFor={`${id}-${option.value}`} className="font-normal">
                  {option.label}
                </Label>
              </div>
            ))}
          </RadioGroup>
          <div className="grid gap-2">
            <Label htmlFor={`${id}-note`}>What would have kept you?</Label>
            <Textarea
              id={`${id}-note`}
              value={note}
              maxLength={NOTE_MAX}
              rows={3}
              onChange={(event) => setNote(event.target.value)}
            />
          </div>
          {offer === "yearly" && saving ? (
            <div className="flex flex-wrap items-center justify-between gap-3 rounded-lg border bg-muted/40 px-3 py-2.5 text-sm">
              <span>
                Pay yearly instead: {formatCents(saving.yearCents)} a year,{" "}
                {formatCents(saving.savedCents)} less.
              </span>
              <Button size="sm" onClick={() => yearly.mutate()} disabled={busy}>
                {yearly.isPending ? <Loader2 className="animate-spin" /> : null}
                Switch to yearly
              </Button>
            </div>
          ) : null}
          {offer === "pause" ? (
            <div className="flex flex-wrap items-center justify-between gap-3 rounded-lg border bg-muted/40 px-3 py-2.5 text-sm">
              <span>
                Take a month free. Your plan stays on, and the month from {ends} costs
                nothing. Billing restarts after it.
              </span>
              <Button size="sm" onClick={() => pause.mutate()} disabled={busy}>
                {pause.isPending ? <Loader2 className="animate-spin" /> : null}
                Take a free month
              </Button>
            </div>
          ) : null}
          <AlertDialogFooter>
            <Button variant="ghost" onClick={() => setOpen(false)} disabled={busy}>
              Keep my plan
            </Button>
            <Button
              variant="outline"
              onClick={() => cancel.mutate()}
              disabled={busy || reason === null}
            >
              {cancel.isPending ? <Loader2 className="animate-spin" /> : null}
              Cancel at period end
            </Button>
          </AlertDialogFooter>
          <p className="text-xs text-muted-foreground">
            {ends ? `Your plan runs until ${ends}. ` : ""}Then your page stays open on
            Free, and nothing is deleted.
          </p>
        </AlertDialogContent>
      </AlertDialog>
    </>
  );
}

/** For a plan set to end: changing one's mind is one click. */
export function KeepPlan() {
  const queryClient = useQueryClient();
  const keep = useMutation({
    mutationFn: keepPlan,
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: QUERY_KEYS.billing });
      toast.success("Your plan carries on.");
    },
    onError: (error) => toast.error(errorMessage(error, LEAVE_ERRORS)),
  });
  return (
    <Button size="sm" onClick={() => keep.mutate()} disabled={keep.isPending}>
      {keep.isPending ? <Loader2 className="animate-spin" /> : null}
      Keep my plan
    </Button>
  );
}
