import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { cn } from "cn";
import { ChevronRight, Copy, Eye, EyeOff, Loader2, Send, Trash2 } from "lucide-react";
import { useState } from "react";
import { toast } from "sonner";

import type { WebhookEndpoint } from "../../bindings/WebhookEndpoint";
import { Button } from "../../components/ui/button";
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from "../../components/ui/collapsible";
import { Input } from "../../components/ui/input";
import { Switch } from "../../components/ui/switch";
import { errorMessage } from "../../lib/error-message";
import { formatDateTime } from "../../lib/format";
import { QUERY_KEYS } from "../../lib/query-keys";
import { billingQuery } from "../billing/billing-api";
import { PlanNotice } from "../billing/plan-notice";
import { PLAN_NAMES } from "../pricing/pricing";
import { ConfirmDialog } from "../shared/confirm-dialog";
import { Field, fieldHelpId } from "../shared/field";
import { StatusBadge } from "../shared/status-badge";
import {
  addWebhook,
  deleteWebhook,
  sendWebhookTest,
  setWebhookActive,
  webhookDeliveriesQuery,
  webhooksQuery,
} from "./webhooks-api";
import { deliveryState, EVENT_LABELS, maskSecret } from "./webhooks";

const ID = "webhook";
const ADD_ERRORS = {
  400: "Use a public address that starts with https://.",
  403: "Webhooks come with Business.",
  409: "You have five webhooks, the most there can be. Delete one first.",
};

function Deliveries({ unid }: { unid: string }) {
  const deliveries = useQuery(webhookDeliveriesQuery(unid));
  if (deliveries.isPending) {
    return <p className="text-sm text-muted-foreground">Loading…</p>;
  }
  if (deliveries.isError) {
    return <p className="text-sm text-destructive">{errorMessage(deliveries.error)}</p>;
  }
  if (deliveries.data.length === 0) {
    return (
      <p className="text-sm text-muted-foreground">
        Nothing sent yet. Book a call, or send a test.
      </p>
    );
  }
  return (
    <ul className="grid gap-1.5 text-sm">
      {deliveries.data.map((delivery) => {
        const state = deliveryState(delivery);
        return (
          <li key={delivery.unid} className="flex flex-wrap items-center gap-x-3 gap-y-1">
            <span className="w-20 font-medium">{EVENT_LABELS[delivery.event]}</span>
            <span className="text-clock text-muted-foreground">
              {formatDateTime(delivery.created_at)}
            </span>
            <StatusBadge value={state.label} statuses={{ [state.label]: state }} />
            {delivery.last_error && !delivery.delivered_at ? (
              <span className="basis-full truncate text-xs text-muted-foreground">
                {delivery.last_error}
              </span>
            ) : null}
          </li>
        );
      })}
    </ul>
  );
}

function EndpointRow({
  endpoint,
  entitled,
}: {
  endpoint: WebhookEndpoint;
  entitled: boolean;
}) {
  const queryClient = useQueryClient();
  const [showSecret, setShowSecret] = useState(false);
  const [confirmingDelete, setConfirmingDelete] = useState(false);
  const [open, setOpen] = useState(false);
  const failed = (what: string) => (error: unknown) =>
    toast.error(
      `${what} ${errorMessage(error, { 403: "Webhooks come with Business." })}`,
    );

  const toggle = useMutation({
    mutationFn: (isActive: boolean) => setWebhookActive(endpoint.unid, isActive),
    onSuccess: (next) =>
      queryClient.setQueryData<WebhookEndpoint[]>(QUERY_KEYS.webhooks, (list) =>
        list?.map((each) => (each.unid === next.unid ? next : each)),
      ),
    onError: failed("It was not changed."),
  });
  const test = useMutation({
    mutationFn: () => sendWebhookTest(endpoint.unid),
    onSuccess: async () => {
      toast.success("Test sent. It shows below in a few seconds.");
      setOpen(true);
      await queryClient.invalidateQueries({
        queryKey: QUERY_KEYS.webhookDeliveries(endpoint.unid),
      });
    },
    onError: failed("The test was not sent."),
  });
  const remove = useMutation({
    mutationFn: () => deleteWebhook(endpoint.unid),
    onSuccess: async () => {
      toast.success("Webhook deleted");
      await queryClient.invalidateQueries({ queryKey: QUERY_KEYS.webhooks });
    },
    onError: failed("It was not deleted."),
  });

  return (
    <li className="grid gap-3 rounded-lg border p-4">
      <div className="flex items-center gap-3">
        <span
          className={cn(
            "min-w-0 flex-1 truncate font-mono text-sm",
            !endpoint.is_active && "text-muted-foreground",
          )}
        >
          {endpoint.url}
        </span>
        <Switch
          checked={endpoint.is_active}
          disabled={toggle.isPending || (!endpoint.is_active && !entitled)}
          onCheckedChange={(checked) => toggle.mutate(checked)}
          aria-label={endpoint.is_active ? "Switch off" : "Switch on"}
        />
      </div>
      <div className="flex flex-wrap items-center gap-2 text-sm">
        <span className="text-muted-foreground">Secret</span>
        <code className="min-w-0 truncate rounded bg-muted px-1.5 py-0.5 text-xs">
          {showSecret ? endpoint.secret : maskSecret(endpoint.secret)}
        </code>
        <Button
          variant="ghost"
          size="icon"
          className="size-7"
          aria-label={showSecret ? "Hide secret" : "Show secret"}
          onClick={() => setShowSecret(!showSecret)}
        >
          {showSecret ? <EyeOff /> : <Eye />}
        </Button>
        <Button
          variant="ghost"
          size="icon"
          className="size-7"
          aria-label="Copy secret"
          onClick={() =>
            navigator.clipboard.writeText(endpoint.secret).then(
              () => toast.success("Secret copied"),
              () => toast.error("The secret could not be copied."),
            )
          }
        >
          <Copy />
        </Button>
        <div className="ml-auto flex gap-2">
          <Button
            variant="outline"
            size="sm"
            disabled={!entitled || test.isPending}
            onClick={() => test.mutate()}
          >
            {test.isPending ? <Loader2 className="animate-spin" /> : <Send />}
            Send test
          </Button>
          <Button
            variant="outline"
            size="sm"
            className="text-destructive"
            disabled={remove.isPending}
            onClick={() => setConfirmingDelete(true)}
          >
            <Trash2 />
            Delete
          </Button>
        </div>
      </div>
      <Collapsible open={open} onOpenChange={setOpen}>
        <CollapsibleTrigger className="flex items-center gap-1 text-sm font-medium outline-none hover:underline focus-visible:ring-[3px] focus-visible:ring-ring/50 [&[data-state=open]>svg]:rotate-90">
          <ChevronRight className="size-4 transition-transform" />
          Recent deliveries
        </CollapsibleTrigger>
        <CollapsibleContent className="overflow-hidden data-[state=closed]:animate-collapsible-up data-[state=open]:animate-collapsible-down">
          <div className="pt-2">{open ? <Deliveries unid={endpoint.unid} /> : null}</div>
        </CollapsibleContent>
      </Collapsible>
      <ConfirmDialog
        open={confirmingDelete}
        onOpenChange={setConfirmingDelete}
        title="Delete this webhook?"
        description={`${endpoint.url} stops hearing about your calls, and its deliveries are deleted.`}
        confirmLabel="Delete"
        destructive
        onConfirm={() => remove.mutate()}
      />
    </li>
  );
}

/** A host's webhooks: add one, switch it, test it, and see what was sent. */
export function WebhooksPanel() {
  const queryClient = useQueryClient();
  const endpoints = useQuery(webhooksQuery);
  const billing = useQuery(billingQuery);
  const [url, setUrl] = useState("");
  const entitled = billing.data?.entitlements.webhooks ?? false;

  const add = useMutation({
    mutationFn: addWebhook,
    onSuccess: async () => {
      setUrl("");
      toast.success("Webhook added. Send a test to check it.");
      await queryClient.invalidateQueries({ queryKey: QUERY_KEYS.webhooks });
    },
    onError: (error) =>
      toast.error(`It was not added. ${errorMessage(error, ADD_ERRORS)}`),
  });

  return (
    <div className="grid gap-5">
      {billing.data && !entitled ? (
        <PlanNotice plan={PLAN_NAMES.business}>
          Webhooks come with Business. Any you added stay saved, and are sent to again
          when you upgrade.
        </PlanNotice>
      ) : null}
      <form
        className="grid gap-2"
        onSubmit={(event) => {
          event.preventDefault();
          add.mutate(url);
        }}
      >
        <Field
          id={`${ID}-url`}
          label="Endpoint URL"
          help="Cadence posts JSON to it for every booked, moved and cancelled call."
        >
          <div className="flex flex-wrap gap-2">
            <Input
              id={`${ID}-url`}
              type="url"
              required
              placeholder="https://example.com/hooks/cadence"
              pattern="https://.*"
              value={url}
              disabled={!entitled}
              aria-describedby={fieldHelpId(`${ID}-url`)}
              onChange={(event) => setUrl(event.target.value)}
              className="min-w-0 flex-1 bg-card"
            />
            <Button type="submit" disabled={!entitled || add.isPending}>
              {add.isPending ? <Loader2 className="animate-spin" /> : null}
              Add webhook
            </Button>
          </div>
        </Field>
      </form>

      {endpoints.data && endpoints.data.length > 0 ? (
        <ul className="grid gap-3">
          {endpoints.data.map((endpoint) => (
            <EndpointRow key={endpoint.unid} endpoint={endpoint} entitled={entitled} />
          ))}
        </ul>
      ) : endpoints.isError ? (
        <p className="text-sm text-destructive">{errorMessage(endpoints.error)}</p>
      ) : null}

      <div className="rounded-lg bg-muted/40 p-4 text-sm text-muted-foreground">
        <p className="font-medium text-foreground">
          Checking a request came from Cadence
        </p>
        <p className="mt-1">
          Each request carries a <code>Cadence-Signature</code> header:{" "}
          <code>t=&lt;unix seconds&gt;,v1=&lt;signature&gt;</code>. The signature is the
          hex HMAC-SHA256 of <code>t</code>, a dot and the raw body, keyed with your
          secret. Answer with any 2xx; anything else is retried for about a day. The{" "}
          <code>id</code> stays the same across retries.
        </p>
      </div>
    </div>
  );
}
