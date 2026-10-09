import { useQuery } from "@tanstack/react-query";
import { cn } from "cn";
import { CalendarDays } from "lucide-react";
import { useEffect, useRef, useState, type CSSProperties } from "react";

import type { BookingPage } from "../../bindings/BookingPage";
import type { TimeSlot } from "../../bindings/TimeSlot";
import { Button } from "../../components/ui/button";
import { Skeleton } from "../../components/ui/skeleton";
import { formatDay } from "../../lib/format";
import { EmptyState } from "../shared/EmptyState";
import { ErrorState } from "../shared/ErrorState";
import type { CountryTimezone } from "../shared/phone/country";
import { BookingCalendar } from "./BookingCalendar";
import { BookingSummary } from "./BookingSummary";
import { publicSlotsQuery } from "./cadence-api";
import { shiftMonth } from "./calendar-grid";
import { TimezoneCombobox } from "./TimezoneCombobox";
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
  { hour12: true, label: "12h" },
  { hour12: false, label: "24h" },
] as const;

const LOADING_ROWS = [0, 1, 2, 3];

/** Times arrive one after another; past this many the rest come in together. */
const STAGGER_MS = 25;
const STAGGER_MAX = 8;

/** Long enough to see the slot fill before the card moves on. */
const PICK_MS = 170;

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
}: {
  slots: LocalSlot[];
  timezone: string;
  hour12: boolean;
  onPick: (slot: TimeSlot) => void;
}) {
  const [picked, setPicked] = useState<string | null>(null);

  function pick(slot: TimeSlot) {
    if (picked !== null) return;
    setPicked(slot.start);
    window.setTimeout(() => onPick(slot), PICK_MS);
  }

  let shown = 0;

  return (
    <div className="grid gap-4 @[760px]:max-h-[22rem] @[760px]:overflow-y-auto @[760px]:pr-1">
      {DAY_PHASES.map((phase) => {
        const inPhase = slots.filter(({ at }) => dayPhase(at.hour) === phase);
        if (inPhase.length === 0) return null;
        const styles = PHASE_STYLES[phase];
        return (
          <section key={phase}>
            <h3 className="mb-2 text-xs font-medium text-muted-foreground">{phase}</h3>
            <ul className="grid grid-cols-3 gap-2 @[760px]:grid-cols-1">
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
                        "relative h-10 w-full overflow-hidden font-display font-narrow text-base tabular-nums hover:border-foreground",
                        isPicked && styles.picked,
                      )}
                      onClick={() => pick(slot)}
                    >
                      <span
                        aria-hidden="true"
                        className={cn(
                          "absolute inset-y-0 left-0 w-[3px] transition-[width] duration-150 ease-out motion-reduce:transition-none",
                          styles.rail,
                          isPicked && "w-full",
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

export function SlotStep({
  slug,
  page,
  timezone,
  onTimezoneChange,
  view,
  onViewChange,
  hour12,
  onHour12Change,
  onPick,
}: {
  slug: string;
  page: BookingPage;
  timezone: CountryTimezone;
  onTimezoneChange: (timezone: CountryTimezone) => void;
  view: CalendarView;
  onViewChange: (view: CalendarView) => void;
  hour12: boolean;
  onHour12Change: (hour12: boolean) => void;
  onPick: (slot: TimeSlot) => void;
}) {
  const query = useQuery(publicSlotsQuery(slug));
  const timesRef = useRef<HTMLDivElement>(null);
  // +1 when the calendar last moved forward, -1 when it moved back.
  const [monthDirection, setMonthDirection] = useState(1);

  const today = zonedTime(new Date(), timezone.iana);
  const slots: LocalSlot[] = (query.data ?? []).map((slot) => ({
    slot,
    at: zonedTime(new Date(slot.start), timezone.iana),
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
    <div className="grid divide-y @[760px]:grid-cols-[15rem_minmax(0,1fr)_13rem] @[760px]:divide-x @[760px]:divide-y-0">
      <div className="p-5 @[760px]:p-6">
        <BookingSummary
          page={page}
          minutes={callMinutes}
          timezone={
            <TimezoneCombobox
              selected={timezone}
              hour12={hour12}
              onSelect={(next) => {
                onTimezoneChange(next);
                onViewChange({ ...view, day: null });
              }}
            />
          }
        />
      </div>

      <div className="p-5 @[760px]:p-6">
        <BookingCalendar
          year={view.year}
          month={view.month}
          direction={monthDirection}
          availableDays={availableDays}
          selectedDay={selectedDay}
          todayDay={isSameMonth(today, view) ? today.day : null}
          canGoPrev={isLaterMonth(
            { ...today, year: view.year, month: view.month },
            today,
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

      <div ref={timesRef} className="p-5 @[760px]:p-6">
        {query.isPending ? (
          <div className="grid gap-2" role="status" aria-label="Loading times">
            {LOADING_ROWS.map((row) => (
              <Skeleton key={row} className="h-10 w-full" />
            ))}
          </div>
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
              description={`${page.host_name} has no open times in the next 7 days. Check back later.`}
            />
          ) : (
            <EmptyState icon={CalendarDays} title="Pick a day to see times" />
          )
        ) : (
          <>
            <div className="mb-4 flex items-center justify-between gap-2">
              <h2 className="text-sm font-semibold">{selectedDayLabel}</h2>
              <div className="flex rounded-md bg-muted p-0.5">
                {CLOCKS.map((clock) => (
                  <button
                    key={clock.label}
                    type="button"
                    aria-pressed={hour12 === clock.hour12}
                    onClick={() => onHour12Change(clock.hour12)}
                    className={cn(
                      "rounded-[5px] px-2 py-0.5 text-xs font-medium text-muted-foreground transition-colors outline-none focus-visible:ring-[3px] focus-visible:ring-ring/50",
                      hour12 === clock.hour12 && "bg-card text-foreground shadow-xs",
                    )}
                  >
                    {clock.label}
                  </button>
                ))}
              </div>
            </div>
            <DayTimes
              // A new day starts with nothing picked.
              key={`${view.year}-${view.month}-${selectedDay}`}
              slots={daySlots}
              timezone={timezone.iana}
              hour12={hour12}
              onPick={onPick}
            />
          </>
        )}
      </div>
    </div>
  );
}
