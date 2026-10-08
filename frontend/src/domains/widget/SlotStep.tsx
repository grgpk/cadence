import { useQuery } from "@tanstack/react-query";
import { cn } from "cn";
import { CalendarDays, Loader2 } from "lucide-react";
import { useState } from "react";

import type { BookingPage } from "../../bindings/BookingPage";
import type { TimeSlot } from "../../bindings/TimeSlot";
import { Button } from "../../components/ui/button";
import { QUERY_KEYS } from "../../lib/query-keys";
import type { CountryTimezone } from "../shared/phone/country";
import { BookingCalendar } from "./BookingCalendar";
import { BookingSummary } from "./BookingSummary";
import { getSlots } from "./cadence-api";
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
                return (
                  <li key={slot.start}>
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
  const query = useQuery({
    queryKey: QUERY_KEYS.publicSlots(slug),
    queryFn: () => getSlots(slug),
  });
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
      : new Intl.DateTimeFormat("en", {
          weekday: "short",
          day: "numeric",
          month: "short",
          timeZone: "UTC",
        }).format(new Date(Date.UTC(view.year, view.month - 1, selectedDay)));

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
          onSelect={(day) => onViewChange({ ...view, day })}
        />
      </div>

      <div className="p-5 @[760px]:p-6">
        {query.isPending ? (
          <p className="flex items-center gap-2 py-4 text-sm text-muted-foreground">
            <Loader2 className="size-4 animate-spin" />
            Loading times
          </p>
        ) : query.isError ? (
          <div className="py-2 text-sm">
            <p className="text-destructive">The times did not load.</p>
            <Button
              type="button"
              variant="outline"
              size="sm"
              className="mt-2"
              onClick={() => void query.refetch()}
            >
              Try again
            </Button>
          </div>
        ) : selectedDay === null || selectedDayLabel === null ? (
          <div className="flex h-full min-h-24 flex-col items-center justify-center gap-2 text-center text-sm text-muted-foreground">
            <CalendarDays className="size-5" />
            {slots.length === 0
              ? `${page.host_name} has no open times in the next 7 days. Check back later.`
              : "Pick a day to see times"}
          </div>
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
                      "rounded-[5px] px-2 py-0.5 text-xs font-medium text-muted-foreground outline-none focus-visible:ring-[3px] focus-visible:ring-ring/50",
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
