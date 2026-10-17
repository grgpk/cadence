import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Loader2, Send } from "lucide-react";
import { toast } from "sonner";

import type { UnfinishedLead } from "../../bindings/UnfinishedLead";
import { Button } from "../../components/ui/button";
import { errorMessage } from "../../lib/error-message";
import { formatDateTime } from "../../lib/format";
import { QUERY_KEYS } from "../../lib/query-keys";
import { PanelCard } from "../shared/panel-card";
import { followUp, unfinishedLeadsQuery } from "./bookings";

function nameOf(lead: UnfinishedLead): string {
  const name = [lead.first_name, lead.last_name].filter(Boolean).join(" ").trim();
  return name || lead.email;
}

function UnfinishedRow({ lead, timeZone }: { lead: UnfinishedLead; timeZone?: string }) {
  const queryClient = useQueryClient();
  const send = useMutation({
    mutationFn: () => followUp(lead.unid),
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: QUERY_KEYS.unfinishedLeads });
      toast.success(`Sent your times to ${lead.email}`);
    },
    onError: (error) => {
      toast.error(
        errorMessage(error, {
          409: `${lead.email} has your times already, or has booked since.`,
        }),
      );
      // The list was stale: show it as it is now.
      void queryClient.invalidateQueries({ queryKey: QUERY_KEYS.unfinishedLeads });
    },
  });
  return (
    <li className="flex flex-wrap items-center justify-between gap-3 py-3">
      <div className="min-w-0">
        <p className="truncate font-medium">{nameOf(lead)}</p>
        <p className="truncate text-sm text-muted-foreground">
          {lead.email}, left {formatDateTime(lead.left_at, timeZone)}
        </p>
      </div>
      <Button
        size="sm"
        variant="outline"
        onClick={() => send.mutate()}
        disabled={send.isPending}
      >
        {send.isPending ? <Loader2 className="animate-spin" /> : <Send />}
        Send my times
      </Button>
    </li>
  );
}

/**
 * Business: people who gave their email on the form and left without a call.
 * Each can be sent the host's times once. Hidden while there is nobody.
 */
export function UnfinishedPanel({ timeZone }: { timeZone?: string }) {
  const query = useQuery(unfinishedLeadsQuery);
  const leads = query.data ?? [];
  if (leads.length === 0) return null;
  return (
    <PanelCard
      title={`Started booking and left (${leads.length})`}
      description="They gave their email but did not pick a time. Send each one your times, once. Your booking form tells people this can happen."
    >
      <ul className="-my-3 divide-y">
        {leads.map((lead) => (
          <UnfinishedRow key={lead.unid} lead={lead} timeZone={timeZone} />
        ))}
      </ul>
    </PanelCard>
  );
}
