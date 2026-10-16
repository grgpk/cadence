import { useQuery } from "@tanstack/react-query";
import { cn } from "cn";
import { CalendarDays } from "lucide-react";
import { useRef, useState, type CSSProperties, type ReactNode } from "react";

import type { TimeSlot } from "../../bindings/TimeSlot";
import { Button } from "../../components/ui/button";
import { CLOCK_OPTIONS } from "../../hooks/use-clock-preference";
import { useNow } from "../../hooks/use-now";
import { shiftMonth, shortDayLabel, todayIn } from "../../lib/civil-day";
import { clockLabel, DAY_PHASES, dayPhase, zonedTime } from "../../lib/zoned-time";
import { PHASE_STYLES } from "../shared/day-phase-styles";
import { EmptyState } from "../shared/empty-state";
import { SegmentedControl } from "../shared/segmented-control";
import { BookingCalendar } from "./booking-calendar";
import { publicSlotsQuery } from "./cadence-api";
import { sharedLength, slotsByDay } from "./slot-days";
import {
  type MonthPosition,
  monthSteps,
  openDaysIn,
  openingMonth,
  selectedDay as selectedDayOf,
} from "./slot-picker-rules";
import { type SlotsWording, slotsStatus } from "./slots-frame";
import { usePick } from "./time-chip";

/** Times arrive one after another; past this many the rest come in together. */
const STAGGER_MS = 25;
const STAGGER_MAX = 8;

/** Fades the last times out, so a list that runs on looks like it does. */
const FADE_OUT =
  "@[760px]:[mask-image:linear-gradient(to_bottom,black_calc(100%-2rem),transparent)]";

function DayTimes({
  slots,
  timezone,
  hour12,
  onPick,
  releaseKey,
}: {
  slots: TimeSlot[];
  timezone: string;
  hour12: boolean;
  onPick: (slot: TimeSlot) => void;
  releaseKey: number;
}) {
  const { picked, pick } = usePick(onPick, releaseKey);
  const timed = slots.map((slot) => ({
    slot,
    hour: zonedTime(new Date(slot.start), timezone).hour,
  }));

  let shown = 0;

  return (
    // Beside the calendar the list scrolls inside the card; the negative margin
    // moves its scrollbar out to the card's edge, away from the buttons.
    <div
      className={cn(
        "grid content-start gap-4 @[760px]:-mr-4 @[760px]:min-h-0 @[760px]:flex-1 @[760px]:overflow-y-auto @[760px]:pr-4 @[760px]:pb-8",
        FADE_OUT,
      )}
    >
      {DAY_PHASES.map((phase) => {
        const inPhase = timed.filter(({ hour }) => dayPhase(hour) === phase);
        if (inPhase.length === 0) return null;
        const styles = PHASE_STYLES[phase];
        return (
          <section key={phase}>
            <h3 className="sticky top-0 z-10 flex items-center gap-2 bg-card pb-2 text-xs font-medium text-muted-foreground">
              <span
                aria-hidden="true"
                className={cn("size-2 rounded-full", styles.rail)}
              />
              {phase}
              <span className="ml-auto tabular-nums">{inPhase.length}</span>
            </h3>
            <ul className="grid grid-cols-2 gap-2 min-[400px]:grid-cols-3 @[760px]:grid-cols-1">
              {inPhase.map(({ slot }) => {
                const isPicked = picked === slot.start;
                const delay = Math.min(shown++, STAGGER_MAX) * STAGGER_MS;
                return (
                  <li
                    key={slot.start}
                    style={
                      {
                        "--step-from": "0.5rem",
                        animationDelay: `${delay}ms`,
                      } as CSSProperties
                    }
                    className="motion-safe:animate-step-in"
                  >
                    <Button
                      type="button"
                      variant="outline"
                      className={cn(
                        "group relative h-11 w-full overflow-hidden text-clock text-[1.0625rem] hover:border-foreground hover:bg-card",
                        isPicked && styles.picked,
                      )}
                      onClick={() => pick(slot)}
                    >
                      <span
                        aria-hidden="true"
                        className={cn(
                          "absolute inset-y-0 left-0 transition-[width] duration-150 ease-out motion-reduce:transition-none",
                          styles.rail,
                          isPicked ? "w-full" : "w-[3px] group-hover:w-2",
                        )}
                      />
                      <span className="relative">
                        {clockLabel(new Date(slot.start), timezone, hour12)}
                      </span>
                    </Button>
                  </li>
                );
              })}
            </ul>
          </section>
        );
      })}
    </div>
  );
}

/**
 * A calendar and the times of the chosen day. With a `summary` it is the
 * booking card's first step; without one, it picks a new time for a call.
 */
export function MonthSlots({
  slug,
  timeZone,
  summary,
  wording,
  chosen,
  onChoose,
  hour12,
  onHour12Change,
  onPick,
  releaseKey,
}: {
  slug: string;
  /** The IANA timezone the times are shown in. */
  timeZone: string;
  /** The column before the calendar, given the call length once it is known. */
  summary?: (callMinutes: number | null) => ReactNode;
  wording: SlotsWording;
  /** The month and day the visitor moved to; `null` opens on the first open month. */
  chosen: MonthPosition | null;
  onChoose: (position: MonthPosition) => void;
  hour12: boolean;
  onHour12Change: (hour12: boolean) => void;
  onPick: (slot: TimeSlot) => void;
  releaseKey: number;
}) {
  const query = useQuery(publicSlotsQuery(slug));
  const timesRef = useRef<HTMLDivElement>(null);
  // +1 when the calendar last moved forward, -1 when it moved back.
  const [monthDirection, setMonthDirection] = useState(1);

  // Refreshed each minute, so the calendar's today moves on at midnight.
  const today = todayIn(timeZone, useNow() ?? new Date());
  const days = slotsByDay(query.data ?? [], timeZone);
  const shown = chosen ?? openingMonth(days, today);
  const day = selectedDayOf(days, shown);
  const daySlots =
    days.find(
      (open) =>
        open.year === shown.year && open.month === shown.month && open.day === day,
    )?.slots ?? [];
  const steps = monthSteps(days, shown);

  function moveMonth(offset: 1 | -1) {
    setMonthDirection(offset);
    onChoose({ ...shiftMonth(shown.year, shown.month, offset), day: null });
  }

  const status = slotsStatus(query, wording, "h-11");

  return (
    <div
      className={cn(
        "grid divide-y @[760px]:divide-x @[760px]:divide-y-0",
        summary
          ? "@[760px]:grid-cols-[15rem_minmax(0,1fr)_14rem]"
          : "@[760px]:grid-cols-[minmax(0,1fr)_14rem]",
      )}
    >
      {summary ? (
        <div className="p-5 @[760px]:p-6">{summary(sharedLength(query.data ?? []))}</div>
      ) : null}

      {/* Without the summary column the calendar would stretch; keep its days a sensible size. */}
      <div className={cn("p-5 @[760px]:p-6", !summary && "mx-auto w-full max-w-md")}>
        <BookingCalendar
          year={shown.year}
          month={shown.month}
          direction={monthDirection}
          availableDays={openDaysIn(days, shown)}
          selectedDay={day}
          todayDay={
            today.year === shown.year && today.month === shown.month ? today.day : null
          }
          canGoPrev={steps.earlier}
          canGoNext={steps.later}
          onPrev={() => moveMonth(-1)}
          onNext={() => moveMonth(1)}
          onSelect={(picked) => {
            onChoose({ ...shown, day: picked });
            // On a phone the times sit below the calendar; this is a no-op beside it.
            requestAnimationFrame(() =>
              timesRef.current?.scrollIntoView({ block: "nearest" }),
            );
          }}
        />
      </div>

      {/* Beside the calendar this column takes the card's height instead of setting it. */}
      <div ref={timesRef} className="@[760px]:relative @[760px]:min-h-[26rem]">
        <div className="flex flex-col p-5 @[760px]:absolute @[760px]:inset-0 @[760px]:p-6">
          {status ??
            (day === null ? (
              <EmptyState icon={CalendarDays} title="Pick a day to see times" />
            ) : (
              <>
                <div className="mb-4 flex items-center justify-between gap-2">
                  <h2
                    aria-live="polite"
                    className="text-sm font-semibold whitespace-nowrap"
                  >
                    {shortDayLabel({ year: shown.year, month: shown.month, day })}
                  </h2>
                  <SegmentedControl
                    label="Clock"
                    size="sm"
                    value={hour12}
                    options={CLOCK_OPTIONS}
                    onChange={onHour12Change}
                  />
                </div>
                <DayTimes
                  // A new day starts with nothing picked.
                  key={`${shown.year}-${shown.month}-${day}`}
                  releaseKey={releaseKey}
                  slots={daySlots}
                  timezone={timeZone}
                  hour12={hour12}
                  onPick={onPick}
                />
              </>
            ))}
        </div>
      </div>
    </div>
  );
}
