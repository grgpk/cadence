import { cn } from "cn";
import { ChevronLeft, ChevronRight } from "lucide-react";
import { useState, type CSSProperties } from "react";

import type { TimeSlot } from "../../bindings/TimeSlot";
import { Button } from "../../components/ui/button";
import {
  addDays,
  dayKey,
  longDayLabel,
  shortWeekday,
  weekDays,
  weekLabel,
  type CivilDay,
  type SlotDay,
} from "./slot-days";
import { SlotsFrame } from "./slots-frame";
import { TimeChip, usePick } from "./time-chip";
import { zonedTime } from "./zoned-time";

const WEEK_DAYS = 7;

/**
 * Fades out where the week runs on: the next days on a phone, which swipes
 * sideways, and the last times beside the summary, which scrolls down.
 */
const FADE_OUT =
  "[mask-image:linear-gradient(to_right,black_calc(100%-1.25rem),transparent)] @[760px]:[mask-image:linear-gradient(to_bottom,black_calc(100%-2rem),transparent)]";

/**
 * The first open day on or after `day`, or the first one at all. A week starts
 * there rather than on a Monday, so it never opens on days already gone.
 */
function firstOpenFrom(days: SlotDay[], day: CivilDay | null): SlotDay | null {
  const from = day ? dayKey(day) : "";
  return days.find((open) => open.key >= from) ?? days[0] ?? null;
}

/**
 * Seven days in a row side by side, each with its open times, for a visitor who wants
 * to see the whole week before choosing.
 */
export function WeekSlots({
  slug,
  timeZone,
  initialDay,
  emptyDescription,
  hour12,
  onHour12Change,
  onPick,
}: {
  slug: string;
  timeZone: string;
  /** The day the visitor last looked at, so the week opens around it. */
  initialDay: CivilDay | null;
  emptyDescription: string;
  hour12: boolean;
  onHour12Change: (hour12: boolean) => void;
  onPick: (slot: TimeSlot) => void;
}) {
  const [chosenWeek, setChosenWeek] = useState<CivilDay | null>(null);
  // +1 when the week last moved forward, -1 when it moved back.
  const [direction, setDirection] = useState(1);
  const { picked, pick } = usePick(onPick);
  const today = dayKey(zonedTime(new Date(), timeZone));

  const shownWeek = (days: SlotDay[]) => chosenWeek ?? firstOpenFrom(days, initialDay);

  return (
    <SlotsFrame
      slug={slug}
      timeZone={timeZone}
      emptyDescription={emptyDescription}
      hour12={hour12}
      onHour12Change={onHour12Change}
      title={(days) => {
        const week = shownWeek(days);
        return week ? weekLabel(week) : "This week";
      }}
      controls={(days) => {
        const week = shownWeek(days);
        if (!week) return null;
        const from = dayKey(week);
        const to = dayKey(addDays(week, WEEK_DAYS));
        // Each move starts on an open day, skipping stretches with nothing to pick.
        const earlier = days.some((day) => day.key < from)
          ? firstOpenFrom(days, addDays(week, -WEEK_DAYS))
          : null;
        const later = days.find((day) => day.key >= to) ?? null;
        const move = (day: SlotDay | null, offset: 1 | -1) => {
          if (!day) return;
          setDirection(offset);
          setChosenWeek(day);
        };
        return (
          <div className="flex items-center gap-1">
            <Button
              type="button"
              variant="ghost"
              size="icon-sm"
              aria-label="Previous week"
              disabled={!earlier}
              onClick={() => move(earlier, -1)}
            >
              <ChevronLeft />
            </Button>
            <Button
              type="button"
              variant="ghost"
              size="icon-sm"
              aria-label="Next week"
              disabled={!later}
              onClick={() => move(later, 1)}
            >
              <ChevronRight />
            </Button>
          </div>
        );
      }}
    >
      {(days) => {
        const week = shownWeek(days);
        if (!week) return null;
        const byKey = new Map(days.map((day) => [day.key, day]));
        return (
          // On a phone the days are columns to swipe through, edge to edge.
          // Beside the summary all seven fit and the week scrolls down inside
          // the card; the negative margin moves that scrollbar to the card's edge.
          <div
            key={dayKey(week)}
            style={{ "--step-from": `${direction}rem` } as CSSProperties}
            className={cn(
              "-mx-5 flex snap-x snap-mandatory scroll-px-5 gap-2 overflow-x-auto px-5 pb-1 motion-safe:animate-step-in @[760px]:mx-0 @[760px]:-mr-4 @[760px]:grid @[760px]:max-h-[26rem] @[760px]:snap-none @[760px]:grid-cols-7 @[760px]:overflow-x-visible @[760px]:overflow-y-auto @[760px]:pr-4 @[760px]:pl-0 @[760px]:pb-8",
              FADE_OUT,
            )}
          >
            {weekDays(week).map((day) => {
              const key = dayKey(day);
              const open = byKey.get(key);
              const isToday = key === today;
              return (
                <section
                  key={key}
                  aria-label={longDayLabel(day)}
                  className="grid w-[5.75rem] shrink-0 snap-start content-start gap-2 @[760px]:w-auto"
                >
                  <h3
                    className={cn(
                      "flex flex-col items-center bg-card pb-1 text-sm @[760px]:sticky @[760px]:top-0 @[760px]:z-10",
                      !open && "text-muted-foreground/60",
                    )}
                  >
                    <span className="text-xs font-medium text-muted-foreground">
                      {shortWeekday(day)}
                    </span>
                    <span
                      className={cn(
                        "flex size-8 items-center justify-center rounded-full font-display text-lg font-semibold tabular-nums",
                        isToday && "ring-1 ring-foreground/25",
                      )}
                    >
                      {day.day}
                    </span>
                    {isToday ? <span className="sr-only">Today</span> : null}
                  </h3>
                  {open ? (
                    <ul className="grid gap-2">
                      {open.slots.map((slot) => (
                        <li key={slot.start}>
                          <TimeChip
                            slot={slot}
                            timeZone={timeZone}
                            hour12={hour12}
                            picked={picked === slot.start}
                            onPick={pick}
                            // Seven columns get narrow before the card is wide.
                            className="@[760px]:text-sm @[960px]:text-[0.9375rem]"
                          />
                        </li>
                      ))}
                    </ul>
                  ) : (
                    <p className="text-center text-sm text-muted-foreground/70">
                      No times
                    </p>
                  )}
                </section>
              );
            })}
          </div>
        );
      }}
    </SlotsFrame>
  );
}
