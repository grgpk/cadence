import { useQuery } from "@tanstack/react-query";
import { cn } from "cn";
import { CalendarDays, Loader2 } from "lucide-react";

import type { GoogleTimeSlot } from "../../bindings/GoogleTimeSlot";
import { Button } from "../../components/ui/button";
import { formatClock } from "../../lib/format";
import { QUERY_KEYS } from "../../lib/query-keys";
import type { CountryTimezone } from "../shared/phone/country";
import { BookingCalendar } from "./BookingCalendar";
import { BookingSummary } from "./BookingSummary";
import { getAvailableSlots } from "./cadence-api";
import { isoDate, shiftMonth } from "./calendar-grid";
import {
  calculateAvailableDays,
  currentDateInTz,
  nextMonthHasDays,
} from "./slot-availability-checker";
import { TimezoneCombobox } from "./TimezoneCombobox";

export type CalendarView = { year: number; month: number; day: number | null };
export type PickedSlot = { date: string; time: string };

const CLOCKS = [
  { hour12: true, label: "12h" },
  { hour12: false, label: "24h" },
] as const;

/** The slot's own calendar date: a late slot can fall on the day after the one picked. */
function slotDate(view: CalendarView & { day: number }, slot: GoogleTimeSlot): string {
  const date = new Date(Date.UTC(view.year, view.month - 1, view.day + slot.day_offset));
  return isoDate(date.getUTCFullYear(), date.getUTCMonth() + 1, date.getUTCDate());
}

function Slots({
  view,
  timezone,
  hour12,
  onPick,
}: {
  view: CalendarView & { day: number };
  timezone: string;
  hour12: boolean;
  onPick: (slot: PickedSlot) => void;
}) {
  const date = isoDate(view.year, view.month, view.day);
  const query = useQuery({
    queryKey: QUERY_KEYS.publicSlots(date, timezone),
    queryFn: () => getAvailableSlots(date, timezone),
  });

  if (query.isPending) {
    return (
      <p className="flex items-center gap-2 py-4 text-sm text-muted-foreground">
        <Loader2 className="size-4 animate-spin" />
        Loading times
      </p>
    );
  }
  if (query.isError) {
    return (
      <div className="py-2 text-sm">
        <p className="text-destructive">Times for this day did not load.</p>
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
    );
  }

  const slots = query.data.filter((slot) => slot.available);
  if (slots.length === 0) {
    return (
      <p className="py-2 text-sm text-muted-foreground">
        No times left on this day. Pick another day.
      </p>
    );
  }

  return (
    <ul className="grid grid-cols-3 gap-2 @[760px]:max-h-[22rem] @[760px]:grid-cols-1 @[760px]:overflow-y-auto @[760px]:pr-1">
      {slots.map((slot) => (
        <li key={`${slot.time}_${slot.day_offset}`}>
          <Button
            type="button"
            variant="outline"
            className="h-10 w-full flex-col gap-0 tabular-nums hover:border-foreground"
            onClick={() => onPick({ date: slotDate(view, slot), time: slot.time })}
          >
            {formatClock(slot.time, hour12)}
            {slot.day_offset !== 0 ? (
              <span className="text-[0.6875rem] leading-none font-normal text-muted-foreground">
                {slot.day_offset > 0 ? "next day" : "previous day"}
              </span>
            ) : null}
          </Button>
        </li>
      ))}
    </ul>
  );
}

export function SlotStep({
  timezone,
  onTimezoneChange,
  view,
  onViewChange,
  hour12,
  onHour12Change,
  onPick,
}: {
  timezone: CountryTimezone;
  onTimezoneChange: (timezone: CountryTimezone) => void;
  view: CalendarView;
  onViewChange: (view: CalendarView) => void;
  hour12: boolean;
  onHour12Change: (hour12: boolean) => void;
  onPick: (slot: PickedSlot) => void;
}) {
  const today = currentDateInTz(timezone.iana);
  const availableDays = calculateAvailableDays(timezone.iana, view.year, view.month);
  const selectedDay =
    view.day !== null && availableDays.includes(view.day) ? view.day : null;
  const canGoPrev =
    view.year > today.year || (view.year === today.year && view.month > today.month);
  const canGoNext = nextMonthHasDays(timezone.iana, view.year, view.month);

  const selectedDayLabel =
    selectedDay === null
      ? null
      : new Intl.DateTimeFormat("en", {
          weekday: "short",
          day: "numeric",
          month: "short",
          timeZone: "UTC",
        }).format(new Date(Date.UTC(view.year, view.month - 1, selectedDay)));

  return (
    <div className="grid divide-y @[760px]:grid-cols-[15rem_minmax(0,1fr)_13rem] @[760px]:divide-x @[760px]:divide-y-0">
      <div className="p-5 @[760px]:p-6">
        <BookingSummary
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
          availableDays={availableDays}
          selectedDay={selectedDay}
          todayDay={
            today.year === view.year && today.month === view.month ? today.day : null
          }
          canGoPrev={canGoPrev}
          canGoNext={canGoNext}
          onPrev={() =>
            onViewChange({ ...shiftMonth(view.year, view.month, -1), day: null })
          }
          onNext={() =>
            onViewChange({ ...shiftMonth(view.year, view.month, 1), day: null })
          }
          onSelect={(day) => onViewChange({ ...view, day })}
        />
      </div>

      <div className="p-5 @[760px]:p-6">
        {selectedDay === null || selectedDayLabel === null ? (
          <div className="flex h-full min-h-24 flex-col items-center justify-center gap-2 text-center text-sm text-muted-foreground">
            <CalendarDays className="size-5" />
            Pick a day to see times
          </div>
        ) : (
          <>
            <div className="mb-4 flex items-center justify-between gap-2">
              <h3 className="text-sm font-semibold">{selectedDayLabel}</h3>
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
            <Slots
              view={{ ...view, day: selectedDay }}
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
