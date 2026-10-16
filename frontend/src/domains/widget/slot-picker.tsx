import { useQuery } from "@tanstack/react-query";
import { useState, type CSSProperties, type ReactNode } from "react";

import type { TimeSlot } from "../../bindings/TimeSlot";
import { publicSlotsQuery } from "./cadence-api";
import { ListSlots } from "./list-slots";
import { MonthSlots } from "./month-slots";
import { type SlotLayout, sharedLength } from "./slot-days";
import type { MonthPosition } from "./slot-picker-rules";
import type { SlotsWording } from "./slots-frame";
import { WeekSlots } from "./week-slots";

/** A switched view slides in a little from the right, the same move as each step. */
const VIEW_IN = { "--step-from": "0.5rem" } as CSSProperties;

/**
 * How a visitor picks an open time, as a month calendar, a week timetable or
 * a list of the soonest times: on the booking page and when moving a call.
 *
 * The picker keeps where the visitor is in time; the page decides the view,
 * the clock and the timezone. A new timezone starts over from the first open
 * month. One pick at a time; change `releaseKey` to let go of one that did not
 * go through.
 */
export function SlotPicker({
  slug,
  timeZone,
  layout,
  hour12,
  onHour12Change,
  onPick,
  releaseKey = 0,
  summary,
  wording,
  windowDays,
}: {
  slug: string;
  /** The IANA timezone the times are shown in. */
  timeZone: string;
  layout: SlotLayout;
  hour12: boolean;
  onHour12Change: (hour12: boolean) => void;
  onPick: (slot: TimeSlot) => void;
  releaseKey?: number;
  /** A column beside the times, given the call length once it is known. */
  summary?: (callMinutes: number | null) => ReactNode;
  wording: SlotsWording;
  /** How far ahead the host takes bookings, when the page knows it. */
  windowDays?: number;
}) {
  const [month, setMonth] = useState<MonthPosition | null>(null);
  // Times move to other days in another timezone: start over from the opening month.
  const [monthZone, setMonthZone] = useState(timeZone);
  if (monthZone !== timeZone) {
    setMonthZone(timeZone);
    setMonth(null);
  }

  const shared = { slug, timeZone, wording, hour12, onHour12Change, onPick, releaseKey };

  if (layout === "month") {
    return (
      // The month card has its own summary column, so all of it comes in.
      <div key="month" style={VIEW_IN} className="motion-safe:animate-step-in">
        <MonthSlots {...shared} summary={summary} chosen={month} onChoose={setMonth} />
      </div>
    );
  }

  const times =
    layout === "week" ? (
      <WeekSlots
        {...shared}
        // The week opens around the day the visitor picked on the calendar.
        initialDay={month?.day ? { ...month, day: month.day } : null}
      />
    ) : (
      <ListSlots {...shared} windowDays={windowDays} />
    );

  return (
    <div
      className={
        summary
          ? "grid divide-y @[760px]:grid-cols-[15rem_minmax(0,1fr)] @[760px]:divide-x @[760px]:divide-y-0"
          : undefined
      }
    >
      {summary ? <SummaryColumn slug={slug} summary={summary} /> : null}
      {/* The summary stays put; only the times come in, so its switch can slide. */}
      <div key={layout} style={VIEW_IN} className="min-w-0 motion-safe:animate-step-in">
        {times}
      </div>
    </div>
  );
}

/** The summary beside the week and list views, told the call length once it is known. */
function SummaryColumn({
  slug,
  summary,
}: {
  slug: string;
  summary: (callMinutes: number | null) => ReactNode;
}) {
  // The same request and cache as the times themselves.
  const slots = useQuery(publicSlotsQuery(slug));
  return (
    <div className="p-5 @[760px]:p-6">{summary(sharedLength(slots.data ?? []))}</div>
  );
}
