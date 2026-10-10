import { useQuery } from "@tanstack/react-query";
import { cn } from "cn";
import { CalendarDays } from "lucide-react";
import { useEffect, useRef, useState, type CSSProperties, type ReactNode } from "react";

import type { TimeSlot } from "../../bindings/TimeSlot";
import { Button } from "../../components/ui/button";
import { Skeleton } from "../../components/ui/skeleton";
import { ApiError } from "../../lib/api";
import { formatDay } from "../../lib/format";
import { EmptyState } from "../shared/EmptyState";
import { ErrorState } from "../shared/ErrorState";
import { SegmentedControl } from "../shared/SegmentedControl";
import { BookingCalendar } from "./BookingCalendar";
import { publicSlotsQuery } from "./cadence-api";
import { shiftMonth } from "./calendar-grid";
import {
  clockLabel,
  DAY_PHASES,
  dayPhase,
  zonedTime,
  type DayPhase,
  type ZonedTime,
} from "./zoned-time";

export type CalendarView = { year: number; month: number; day: number | null };

const CLOCKS = [
  { value: true, label: "12h" },
  { value: false, label: "24h" },
] as const;

const LOADING_ROWS = [0, 1, 2, 3];

/** Times arrive one after another; past this many the rest come in together. */
const STAGGER_MS = 25;
const STAGGER_MAX = 8;

/** Long enough to see the slot fill before the card moves on. */
const PICK_MS = 170;

/** Fades the last times out, so a list that runs on looks like it does. */
const FADE_OUT =
  "@[760px]:[mask-image:linear-gradient(to_bottom,black_calc(100%-2rem),transparent)]";

/** Each part of the day has its own color, so a time reads as morning or evening at a glance. */
export const PHASE_STYLES: Record<DayPhase, { rail: string; picked: string }> = {
  Morning: { rail: "bg-dawn", picked: "text-day-foreground" },
  Afternoon: { rail: "bg-noon", picked: "text-day-foreground" },
  Evening: { rail: "bg-dusk", picked: "text-dusk-foreground" },
};

type LocalSlot = { slot: TimeSlot; at: ZonedTime };

function isSameMonth(at: ZonedTime, view: CalendarView): boolean {
  return at.year === view.year && at.month === view.month;
}

function isLaterMonth(at: ZonedTime, view: CalendarView): boolean {
  return at.year > view.year || (at.year === view.year && at.month > view.month);
}

function DayTimes({
  slots,
  timezone,
  hour12,
  onPick,
  resetKey,
}: {
  slots: LocalSlot[];
  timezone: string;
  hour12: boolean;
  onPick: (slot: TimeSlot) => void;
  /** Changes when a picked time did not go through, to let go of it. */
  resetKey: number;
}) {
  const [picked, setPicked] = useState<string | null>(null);
  const timer = useRef<number | undefined>(undefined);
  // A day that is left before the fill finishes must not report its pick.
  useEffect(() => () => window.clearTimeout(timer.current), []);
  // biome-ignore lint/correctness/useExhaustiveDependencies: the key is the signal, not a value read here
  useEffect(() => setPicked(null), [resetKey]);

  function pick(slot: TimeSlot) {
    if (picked !== null) return;
    setPicked(slot.start);
    timer.current = window.setTimeout(() => onPick(slot), PICK_MS);
  }

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
        const inPhase = slots.filter(({ at }) => dayPhase(at.hour) === phase);
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
 * A calendar and the times of the chosen day, for a host's open slots. With a
 * `summary` it is the first step of booking; without one, it picks a new time
 * for a call that already exists.
 */
export function SlotStep({
  slug,
  timeZone,
  summary,
  emptyDescription,
  pausedDescription,
  view,
  onViewChange,
  hour12,
  onHour12Change,
  onPick,
  pickKey = 0,
}: {
  slug: string;
  /** The IANA timezone the times are shown in. */
  timeZone: string;
  /** The column before the calendar, given the call length once it is known. */
  summary?: (callMinutes: number | null) => ReactNode;
  /** Said when the host has no open times at all. */
  emptyDescription: string;
  /** Said when the host's page stopped taking bookings; defaults to the above. */
  pausedDescription?: string;
  view: CalendarView;
  onViewChange: (view: CalendarView) => void;
  hour12: boolean;
  onHour12Change: (hour12: boolean) => void;
  onPick: (slot: TimeSlot) => void;
  /** Change it to let go of a picked time, when the pick did not go through. */
  pickKey?: number;
}) {
  const query = useQuery(publicSlotsQuery(slug));
  const timesRef = useRef<HTMLDivElement>(null);
  // +1 when the calendar last moved forward, -1 when it moved back.
  const [monthDirection, setMonthDirection] = useState(1);

  const today = zonedTime(new Date(), timeZone);
  const slots: LocalSlot[] = (query.data ?? []).map((slot) => ({
    slot,
    at: zonedTime(new Date(slot.start), timeZone),
  }));
  const inMonth = slots.filter(({ at }) => isSameMonth(at, view));
  const availableDays = [...new Set(inMonth.map(({ at }) => at.day))];
  // Until the visitor picks, show the first day that has times.
  const selectedDay =
    view.day !== null && availableDays.includes(view.day)
      ? view.day
      : (availableDays[0] ?? null);
  const daySlots = inMonth.filter(({ at }) => at.day === selectedDay);
  const lengths = new Set(slots.map(({ slot }) => slot.minutes));
  const callMinutes = lengths.size === 1 ? (slots[0]?.slot.minutes ?? null) : null;

  const selectedDayLabel =
    selectedDay === null
      ? null
      : formatDay(
          new Date(Date.UTC(view.year, view.month - 1, selectedDay)).toISOString(),
          "UTC",
        );

  // The calendar opens on this month. If the first open time is in a later one,
  // start there instead of on a month with nothing to pick.
  const firstSlot = slots[0]?.at;
  const opened = useRef(false);
  useEffect(() => {
    if (opened.current || !firstSlot) return;
    opened.current = true;
    if (isLaterMonth(firstSlot, view)) {
      onViewChange({ year: firstSlot.year, month: firstSlot.month, day: null });
    }
  });

  function moveMonth(offset: 1 | -1) {
    setMonthDirection(offset);
    onViewChange({ ...shiftMonth(view.year, view.month, offset), day: null });
  }

  return (
    <div
      className={cn(
        "grid divide-y @[760px]:divide-x @[760px]:divide-y-0",
        summary
          ? "@[760px]:grid-cols-[15rem_minmax(0,1fr)_14rem]"
          : "@[760px]:grid-cols-[minmax(0,1fr)_14rem]",
      )}
    >
      {summary ? <div className="p-5 @[760px]:p-6">{summary(callMinutes)}</div> : null}

      {/* Without the summary column the calendar would stretch; keep its days a sensible size. */}
      <div className={cn("p-5 @[760px]:p-6", !summary && "mx-auto w-full max-w-md")}>
        <BookingCalendar
          year={view.year}
          month={view.month}
          direction={monthDirection}
          availableDays={availableDays}
          selectedDay={selectedDay}
          todayDay={isSameMonth(today, view) ? today.day : null}
          // Only back to a month that has times; earlier ones have nothing to pick.
          canGoPrev={slots.some(({ at }) =>
            isLaterMonth({ ...at, year: view.year, month: view.month }, at),
          )}
          canGoNext={slots.some(({ at }) => isLaterMonth(at, view))}
          onPrev={() => moveMonth(-1)}
          onNext={() => moveMonth(1)}
          onSelect={(day) => {
            onViewChange({ ...view, day });
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
          {query.isPending ? (
            <div className="grid gap-2" role="status" aria-label="Loading times">
              {LOADING_ROWS.map((row) => (
                <Skeleton key={row} className="h-11 w-full" />
              ))}
            </div>
          ) : query.error instanceof ApiError && query.error.isNotFound ? (
            // The page was paused after this call was booked.
            <EmptyState
              icon={CalendarDays}
              title="No open times"
              description={pausedDescription ?? emptyDescription}
            />
          ) : query.isError ? (
            <ErrorState
              title="The times did not load"
              error={query.error}
              onRetry={() => void query.refetch()}
            />
          ) : selectedDay === null || selectedDayLabel === null ? (
            slots.length === 0 ? (
              <EmptyState
                icon={CalendarDays}
                title="No open times"
                description={emptyDescription}
              />
            ) : (
              <EmptyState icon={CalendarDays} title="Pick a day to see times" />
            )
          ) : (
            <>
              <div className="mb-4 flex items-center justify-between gap-2">
                <h2
                  aria-live="polite"
                  className="text-sm font-semibold whitespace-nowrap"
                >
                  {selectedDayLabel}
                </h2>
                <SegmentedControl
                  label="Clock"
                  size="sm"
                  value={hour12}
                  options={CLOCKS}
                  onChange={onHour12Change}
                />
              </div>
              <DayTimes
                // A new day starts with nothing picked.
                key={`${view.year}-${view.month}-${selectedDay}`}
                resetKey={pickKey}
                slots={daySlots}
                timezone={timeZone}
                hour12={hour12}
                onPick={onPick}
              />
            </>
          )}
        </div>
      </div>
    </div>
  );
}
