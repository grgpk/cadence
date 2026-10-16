import type { Entitlements } from "../../bindings/Entitlements";
import type { EventType } from "../../bindings/EventType";
import type { EventTypeInput } from "../../bindings/EventTypeInput";
import { lengthLabel } from "../../lib/format";

/** The same limits the API enforces. */
export const NOTICE_HOURS = { min: 0, max: 168 };
export const WINDOW_DAYS = { min: 1, max: 90 };
export const BUFFER_MINUTES = { min: 0, max: 120 };
export const SLUG_PATTERN = "[a-z0-9]+(-[a-z0-9]+)*";

/** What the editor saves of a type: everything but its id and plan state. */
export function inputOf({
  unid: _unid,
  bookable: _bookable,
  ...input
}: EventType): EventTypeInput {
  return input;
}

/** `base`, or `base-2`, `base-3`… whichever the host does not use yet. */
export function freeSlug(base: string, taken: readonly string[]): string {
  if (!taken.includes(base)) return base;
  let n = 2;
  while (taken.includes(`${base}-${n}`)) n += 1;
  return `${base}-${n}`;
}

/** A new kind of call: a 30-minute call at a link of its own, on with the usual rules. */
export function newTypeInput(existing: readonly EventType[]): EventTypeInput {
  return {
    slug: freeSlug(
      "new-call",
      existing.map((kind) => kind.slug),
    ),
    title: "New call",
    description: "",
    duration_minutes: 30,
    buffer_minutes: 0,
    minimum_notice_hours: 4,
    booking_window_days: 7,
    ask_currently_working_on: false,
    ask_how_heard_about_us: false,
    ask_what_stopping_you: false,
    ask_investment_comfort: false,
    ask_urgency_level: false,
    is_active: true,
  };
}

/** "30 min", or that the host's hours decide. */
export function lengthText(minutes: number | null): string {
  return minutes === null ? "Length from your hours" : lengthLabel(minutes);
}

/** Whether the plan lets the host add one more type. */
export function canAddType(
  entitlements: Pick<Entitlements, "max_event_types">,
  count: number,
): boolean {
  return entitlements.max_event_types === null || count < entitlements.max_event_types;
}
