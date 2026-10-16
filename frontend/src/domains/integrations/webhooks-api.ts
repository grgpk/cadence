import { queryOptions } from "@tanstack/react-query";

import type { WebhookDelivery } from "../../bindings/WebhookDelivery";
import type { WebhookEndpoint } from "../../bindings/WebhookEndpoint";
import { api } from "../../lib/api";
import { QUERY_KEYS } from "../../lib/query-keys";

const WEBHOOKS_PATH = "/api/host/webhooks";

export const webhooksQuery = queryOptions({
  queryKey: QUERY_KEYS.webhooks,
  queryFn: () => api<WebhookEndpoint[]>(WEBHOOKS_PATH),
});

export function webhookDeliveriesQuery(unid: string) {
  return queryOptions({
    queryKey: QUERY_KEYS.webhookDeliveries(unid),
    queryFn: () => api<WebhookDelivery[]>(`${WEBHOOKS_PATH}/${unid}/deliveries`),
  });
}

export function addWebhook(url: string): Promise<WebhookEndpoint> {
  return api<WebhookEndpoint>(WEBHOOKS_PATH, {
    method: "POST",
    body: JSON.stringify({ url }),
  });
}

export function setWebhookActive(
  unid: string,
  isActive: boolean,
): Promise<WebhookEndpoint> {
  return api<WebhookEndpoint>(`${WEBHOOKS_PATH}/${unid}`, {
    method: "PUT",
    body: JSON.stringify({ is_active: isActive }),
  });
}

export function deleteWebhook(unid: string): Promise<void> {
  return api<void>(`${WEBHOOKS_PATH}/${unid}`, { method: "DELETE" });
}

export function sendWebhookTest(unid: string): Promise<void> {
  return api<void>(`${WEBHOOKS_PATH}/${unid}/test`, { method: "POST" });
}
