import { useMutation, useQueryClient } from "@tanstack/react-query";
import { MoreHorizontal } from "lucide-react";
import { toast } from "sonner";

import type { AdminPlan } from "../../bindings/AdminPlan";
import type { Plan } from "../../bindings/Plan";
import { Button } from "../../components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "../../components/ui/dropdown-menu";
import { errorMessage } from "../../lib/error-message";
import { QUERY_KEYS } from "../../lib/query-keys";
import { PLAN, PLAN_NAMES } from "../pricing/pricing";
import { compPlan } from "./billing-api";
import { isBilled, STATUS } from "./billing";

/** An admin's plan actions for one host. A plan Stripe bills is changed in Stripe. */
export function CompMenu({ row, endpoint }: { row: AdminPlan; endpoint: string }) {
  const queryClient = useQueryClient();
  const change = useMutation({
    mutationFn: (plan: Plan | null) => compPlan(row.user_unid, { plan }),
    onSuccess: async (_, plan) => {
      await queryClient.invalidateQueries({ queryKey: QUERY_KEYS.adminData(endpoint) });
      toast.success(
        plan
          ? `${row.full_name} has ${PLAN_NAMES[plan]} for free`
          : `${row.full_name}'s free plan was taken back`,
      );
    },
    onError: (error) =>
      toast.error(
        errorMessage(error, { 409: "This host pays through Stripe. Change it there." }),
      ),
  });

  if (isBilled(row)) {
    return <span className="text-sm text-muted-foreground">In Stripe</span>;
  }
  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button
          variant="ghost"
          size="icon"
          aria-label={`Plan actions for ${row.full_name}`}
          disabled={change.isPending}
        >
          <MoreHorizontal />
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end">
        {[PLAN.PRO, PLAN.BUSINESS].map((plan) => (
          <DropdownMenuItem
            key={plan}
            disabled={row.status === STATUS.COMPED && row.plan === plan}
            onSelect={() => change.mutate(plan)}
          >
            Give {PLAN_NAMES[plan]} for free
          </DropdownMenuItem>
        ))}
        {row.status === STATUS.COMPED ? (
          <DropdownMenuItem onSelect={() => change.mutate(null)}>
            Take back the free plan
          </DropdownMenuItem>
        ) : null}
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
