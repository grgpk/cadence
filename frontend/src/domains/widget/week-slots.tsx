import { cn } from "cn";
import { ChevronLeft, ChevronRight } from "lucide-react";
import { Fragment, useState, type CSSProperties } from "react";

import type { TimeSlot } from "../../bindings/TimeSlot";
import { Button } from "../../components/ui/button";
import { useNow } from "../../hooks/use-now";
import {
  type CivilDay,
  dayKey,
  longDayLabel,
  shortWeekday,
  todayIn,
  weekFrom,
  weekLabel,
} from "../../lib/civil-day";
import { dayPhase } from "../../lib/zoned-time";
import { PHASE_STYLES } from "../shared/day-phase-styles";
import { type SlotDay, weekTimetable } from "./slot-days";
import { weekAround, weekSteps } from "./slot-picker-rules";
import { SlotsFrame, type SlotsWording } from "./slots-frame";
import { TimeChip, usePick } from "./time-chip";

/**
 * Fades out where the week runs on: the next days on a phone, which swipes
 * sideways, and the last times beside the summary, which scrolls down.
 */
const FADE_OUT =
  "[mask-image:linear-gradient(to_right,black_calc(100%-1.25rem),transparent)] @[760px]:[mask-image:linear-gradient(to_bottom,black_calc(100%-2rem),transparent)]";

/**
 * Seven days in a row as a timetable, for a visitor who wants to compare days
 * before choosing: each row is one time of day, so "Tuesday at 2" is a glance
 * across.
 */
export function WeekSlots({
  slug,
  timeZone,
  initialDay,
  wording,
  hour12,
  onHour12Change,
  onPick,
  releaseKey,
}: {
  slug: string;
  timeZone: string;
  /** The day the visitor last looked at, so the week opens around it. */
  initialDay: CivilDay | null;
  wording: SlotsWording;
  hour12: boolean;
  onHour12Change: (hour12: boolean) => void;
  onPick: (slot: TimeSlot) => void;
  /** Change it to let go of a picked time that did not go through. */
  releaseKey: number;
}) {
  const [chosenWeek, setChosenWeek] = useState<CivilDay | null>(null);
  // +1 when the week last moved forward, -1 when it moved back.
  const [direction, setDirection] = useState(1);
  const { picked, pick } = usePick(onPick, releaseKey);
  // Refreshed each minute, so "today" moves on at midnight.
  const today = dayKey(todayIn(timeZone, useNow() ?? new Date()));

  const shownWeek = (days: SlotDay[]) => chosenWeek ?? weekAround(days, initialDay);

  return (
    <SlotsFrame
      slug={slug}
      timeZone={timeZone}
      wording={wording}
      hour12={hour12}
      onHour12Change={onHour12Change}
      title={(days) => {
        const week = shownWeek(days);
        return week ? weekLabel(week) : "This week";
      }}
      controls={(days) => {
        const week = shownWeek(days);
        if (!week) return null;
        const { earlier, later } = weekSteps(days, week);
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
        const start = shownWeek(days);
        if (!start) return null;
        const week = weekFrom(start);
        const rows = weekTimetable(days, week, timeZone);
        const openColumns = week.map((_, column) =>
          rows.some((row) => row.cells[column] !== null),
        );
        return (
          // On a phone the days are columns to swipe through, edge to edge.
          // Beside the summary all seven fit and the week scrolls down inside
          // the card; the negative margin moves that scrollbar to the card's edge.
          <div
            key={dayKey(start)}
            style={{ "--step-from": `${direction}rem` } as CSSProperties}
            className={cn(
              "-mx-5 snap-x snap-mandatory scroll-px-5 overflow-x-auto px-5 pb-1 motion-safe:animate-step-in @[760px]:mx-0 @[760px]:-mr-4 @[760px]:max-h-[23rem] @[760px]:snap-none @[760px]:overflow-y-auto @[760px]:pr-4 @[760px]:pl-0 @[760px]:pb-8",
              FADE_OUT,
            )}
          >
            {/* One row per time of day, so a time lines up across the week. */}
            <div className="grid w-max grid-cols-[repeat(7,5.75rem)] gap-2 @[760px]:w-auto @[760px]:grid-cols-7">
              {week.map((day, column) => {
                const key = dayKey(day);
                const isToday = key === today;
                return (
                  <h3
                    key={key}
                    className={cn(
                      "flex snap-start flex-col items-center bg-card pb-1 text-sm @[760px]:sticky @[760px]:top-0 @[760px]:z-10",
                      !openColumns[column] && "text-muted-foreground/60",
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
                );
              })}
              {rows.map((row, index) => {
                const phase = dayPhase(Math.floor(row.minutes / 60));
                const startsPhase =
                  index === 0 ||
                  dayPhase(Math.floor((rows[index - 1]?.minutes ?? 0) / 60)) !== phase;
                return (
                  <Fragment key={row.minutes}>
                    {startsPhase ? (
                      <p className="col-span-full pt-2 text-xs font-medium text-muted-foreground">
                        {/* Stays at the left edge while a phone swipes through days. */}
                        <span className="sticky left-0 inline-flex items-center gap-2">
                          <span
                            aria-hidden="true"
                            className={cn(
                              "size-2 rounded-full",
                              PHASE_STYLES[phase].rail,
                            )}
                          />
                          {phase}
                        </span>
                      </p>
                    ) : null}
                    {row.cells.map((slot, column) =>
                      slot ? (
                        <TimeChip
                          key={slot.start}
                          slot={slot}
                          timeZone={timeZone}
                          hour12={hour12}
                          picked={picked === slot.start}
                          onPick={pick}
                          dayLabel={longDayLabel(week[column] ?? start)}
                          // Seven columns get narrow before the card is wide.
                          className="@[760px]:text-sm @[960px]:text-[0.9375rem]"
                        />
                      ) : (
                        // Not open at this time on this day; a column is a fixed day of this week.
                        <span
                          key={column}
                          aria-hidden="true"
                          className="flex h-10 items-center justify-center"
                        >
                          <span className="h-px w-3 bg-border" />
                        </span>
                      ),
                    )}
                  </Fragment>
                );
              })}
            </div>
          </div>
        );
      }}
    </SlotsFrame>
  );
}
