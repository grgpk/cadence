import { cn } from "cn";
import { ChevronDown } from "lucide-react";
import { useState } from "react";

import type { TimeSlot } from "../../bindings/TimeSlot";
import { Button } from "../../components/ui/button";
import { relativeDayLabel } from "./slot-days";
import { SlotsFrame } from "./slots-frame";
import { TimeChip, usePick } from "./time-chip";
import { zonedTime } from "./zoned-time";

/** Days shown at first, and added by each "Show more days". */
const DAYS_PER_PAGE = 5;

/**
 * A day shows two rows of times until it is opened, so the soonest days all
 * fit on screen. Rows hold 3 times on a phone, 4 beside the summary, 6 when
 * the card is wide; each class hides the times past two rows at that width.
 */
function beyondTwoRows(index: number): string | undefined {
  if (index < 6) return undefined;
  if (index < 8) return "hidden @[760px]:block";
  if (index < 12) return "hidden @[960px]:block";
  return "hidden";
}

/** The "Show all times" button, at the widths where some times are hidden. */
function showAllWhen(count: number): string | null {
  if (count > 12) return "";
  if (count > 8) return "@[960px]:hidden";
  if (count > 6) return "@[760px]:hidden";
  return null;
}

/**
 * Open times day after day, soonest first, so the earliest time is one click
 * away.
 */
export function ListSlots({
  slug,
  timeZone,
  emptyDescription,
  hour12,
  onHour12Change,
  onPick,
}: {
  slug: string;
  timeZone: string;
  emptyDescription: string;
  hour12: boolean;
  onHour12Change: (hour12: boolean) => void;
  onPick: (slot: TimeSlot) => void;
}) {
  const [shownDays, setShownDays] = useState(DAYS_PER_PAGE);
  const [openDays, setOpenDays] = useState<string[]>([]);
  const { picked, pick } = usePick(onPick);
  const today = zonedTime(new Date(), timeZone);

  return (
    <SlotsFrame
      slug={slug}
      timeZone={timeZone}
      emptyDescription={emptyDescription}
      hour12={hour12}
      onHour12Change={onHour12Change}
      title={() => "Soonest open times"}
    >
      {(days) => (
        <div className="grid gap-6">
          {days.slice(0, shownDays).map((day) => {
            const open = openDays.includes(day.key);
            const showAll = open ? null : showAllWhen(day.slots.length);
            return (
              <section key={day.key} className="grid gap-2.5">
                <h3 className="flex items-baseline gap-2 text-sm font-semibold">
                  {relativeDayLabel(day, today)}
                  <span className="ml-auto text-xs font-normal text-muted-foreground tabular-nums">
                    {day.slots.length} {day.slots.length === 1 ? "time" : "times"}
                  </span>
                </h3>
                <ul className="grid grid-cols-3 gap-2 @[760px]:grid-cols-4 @[960px]:grid-cols-6">
                  {day.slots.map((slot, index) => (
                    <li
                      key={slot.start}
                      className={open ? undefined : beyondTwoRows(index)}
                    >
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
                {showAll === null ? null : (
                  <Button
                    type="button"
                    variant="ghost"
                    size="sm"
                    className={cn(
                      "-ml-2.5 justify-self-start text-muted-foreground",
                      showAll,
                    )}
                    onClick={() => setOpenDays([...openDays, day.key])}
                  >
                    <ChevronDown />
                    Show all {day.slots.length} times
                  </Button>
                )}
              </section>
            );
          })}
          {days.length > shownDays ? (
            <Button
              type="button"
              variant="outline"
              className="justify-self-start"
              onClick={() => setShownDays(shownDays + DAYS_PER_PAGE)}
            >
              <ChevronDown />
              Show more days
            </Button>
          ) : null}
        </div>
      )}
    </SlotsFrame>
  );
}
