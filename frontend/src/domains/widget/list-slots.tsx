import { cn } from "cn";
import { ChevronDown } from "lucide-react";
import { useState } from "react";

import type { TimeSlot } from "../../bindings/TimeSlot";
import { Button } from "../../components/ui/button";
import { useNow } from "../../hooks/use-now";
import { relativeDayLabel, todayIn } from "../../lib/civil-day";
import { DAYS_PER_PAGE, listPage } from "./slot-picker-rules";
import { SlotsFrame, type SlotsWording } from "./slots-frame";
import { TimeChip, usePick } from "./time-chip";

/**
 * Beside the summary the list scrolls inside the card, as the week does, so
 * every view keeps the card about the height of the month calendar. The
 * negative margin moves the scrollbar out to the card's edge, and the last
 * times fade out.
 */
const SCROLL_INSIDE =
  "@[760px]:-mr-4 @[760px]:max-h-[23rem] @[760px]:overflow-y-auto @[760px]:pr-4 @[760px]:pb-8 @[760px]:[mask-image:linear-gradient(to_bottom,black_calc(100%-2rem),transparent)]";

/**
 * Open times day after day, soonest first, so the earliest time is one click
 * away.
 */
export function ListSlots({
  slug,
  timeZone,
  wording,
  windowDays,
  hour12,
  onHour12Change,
  onPick,
  releaseKey,
}: {
  slug: string;
  timeZone: string;
  wording: SlotsWording;
  /** How far ahead the host takes bookings, for the line under the last day. */
  windowDays?: number;
  hour12: boolean;
  onHour12Change: (hour12: boolean) => void;
  onPick: (slot: TimeSlot) => void;
  /** Change it to let go of a picked time that did not go through. */
  releaseKey: number;
}) {
  const [shownDays, setShownDays] = useState(DAYS_PER_PAGE);
  const { picked, pick } = usePick(onPick, releaseKey);
  // Refreshed each minute, so "Today" and "Tomorrow" move on at midnight.
  const today = todayIn(timeZone, useNow() ?? new Date());

  return (
    <SlotsFrame
      slug={slug}
      timeZone={timeZone}
      wording={wording}
      hour12={hour12}
      onHour12Change={onHour12Change}
      title={() => "Soonest open times"}
    >
      {(days) => {
        const page = listPage(days, shownDays);
        return (
          <div className={cn("grid content-start gap-6", SCROLL_INSIDE)}>
            {page.days.map((day) => (
              <section key={day.key} className="grid gap-2.5">
                <h3 className="flex items-baseline gap-2 bg-card text-sm font-semibold @[760px]:sticky @[760px]:top-0 @[760px]:z-10 @[760px]:pb-1">
                  {relativeDayLabel(day, today)}
                  <span className="ml-auto text-xs font-normal text-muted-foreground tabular-nums">
                    {day.slots.length} {day.slots.length === 1 ? "time" : "times"}
                  </span>
                </h3>
                <ul className="grid grid-cols-3 gap-2 @[760px]:grid-cols-4 @[960px]:grid-cols-6">
                  {day.slots.map((slot) => (
                    <li key={slot.start}>
                      <TimeChip
                        slot={slot}
                        timeZone={timeZone}
                        hour12={hour12}
                        picked={picked === slot.start}
                        onPick={pick}
                      />
                    </li>
                  ))}
                </ul>
              </section>
            ))}
            {page.more ? (
              <Button
                type="button"
                variant="outline"
                className="justify-self-start"
                onClick={() => setShownDays(shownDays + DAYS_PER_PAGE)}
              >
                <ChevronDown />
                Show more days
              </Button>
            ) : (
              <p className="text-sm text-muted-foreground">
                {windowDays === undefined
                  ? "Those are all the open times."
                  : `Those are all the open times in the next ${windowDays} days.`}
              </p>
            )}
          </div>
        );
      }}
    </SlotsFrame>
  );
}
