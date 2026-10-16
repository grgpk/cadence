import { queryOptions } from "@tanstack/react-query";

import type { EventType } from "../../bindings/EventType";
import type { EventTypeInput } from "../../bindings/EventTypeInput";
import { api } from "../../lib/api";
import { QUERY_KEYS } from "../../lib/query-keys";

const EVENT_TYPES_PATH = "/api/host/event-types";

export const eventTypesQuery = queryOptions({
  queryKey: QUERY_KEYS.eventTypes,
  queryFn: () => api<EventType[]>(EVENT_TYPES_PATH),
});

export function createEventType(input: EventTypeInput): Promise<EventType> {
  return api<EventType>(EVENT_TYPES_PATH, {
    method: "POST",
    body: JSON.stringify(input),
  });
}

export function saveEventType(unid: string, input: EventTypeInput): Promise<EventType> {
  return api<EventType>(`${EVENT_TYPES_PATH}/${unid}`, {
    method: "PUT",
    body: JSON.stringify(input),
  });
}

export function deleteEventType(unid: string): Promise<void> {
  return api<void>(`${EVENT_TYPES_PATH}/${unid}`, { method: "DELETE" });
}
