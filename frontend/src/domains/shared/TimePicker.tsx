import { Check, ChevronDown } from "lucide-react";
import { useState } from "react";

import { Button } from "../../components/ui/button";
import {
  Command,
  CommandEmpty,
  CommandGroup,
  CommandInput,
  CommandItem,
  CommandList,
} from "../../components/ui/command";
import { Popover, PopoverContent, PopoverTrigger } from "../../components/ui/popover";
import { formatClock } from "../../lib/format";

const STEP_MINUTES = 15;
const DAY_MINUTES = 24 * 60;

function toMinutes(time: string): number {
  const [hours = 0, minutes = 0] = time.split(":").map(Number);
  return hours * 60 + minutes;
}

function toTime(minutes: number): string {
  return `${String(Math.floor(minutes / 60)).padStart(2, "0")}:${String(minutes % 60).padStart(2, "0")}`;
}

/** Every quarter hour of the day as "HH:MM". */
const TIMES = Array.from({ length: DAY_MINUTES / STEP_MINUTES }, (_, index) =>
  toTime(index * STEP_MINUTES),
);

/** "45 min", "8 h", "7 h 30 min" */
function lengthLabel(minutes: number): string {
  const hours = Math.floor(minutes / 60);
  const rest = minutes % 60;
  return [hours ? `${hours} h` : "", rest ? `${rest} min` : ""].filter(Boolean).join(" ");
}

/** Whether this device writes times as "5:00 PM". */
export function prefersHour12(): boolean {
  return (
    new Intl.DateTimeFormat(undefined, { hour: "numeric" }).resolvedOptions().hour12 ??
    false
  );
}

/** A time of day, picked from a list or typed. Reads and writes a wall-clock "HH:MM". */
export function TimePicker({
  id,
  value,
  onChange,
  after,
  invalid = false,
}: {
  id: string;
  value: string;
  onChange: (time: string) => void;
  /** Offer only later times, each with how long after this one it is. */
  after?: string;
  invalid?: boolean;
}) {
  const [open, setOpen] = useState(false);
  const hour12 = prefersHour12();
  const floor = after === undefined ? -1 : toMinutes(after);
  const times = TIMES.filter((time) => toMinutes(time) > floor);
  // Both spellings, so "17", "5:30" and "5 pm" all find the same time.
  const searchValue = (time: string) =>
    `${formatClock(time, true)} ${formatClock(time, false)}`;

  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger asChild>
        <Button
          id={id}
          type="button"
          variant="outline"
          role="combobox"
          aria-expanded={open}
          aria-invalid={invalid}
          className="w-32 justify-between bg-card font-normal tabular-nums"
        >
          {formatClock(value, hour12)}
          <ChevronDown className="text-muted-foreground" />
        </Button>
      </PopoverTrigger>
      <PopoverContent align="start" className="w-60 p-0">
        {/* Starting on the current time also scrolls the list to it. */}
        <Command defaultValue={searchValue(value)}>
          <CommandInput placeholder="Type a time" />
          <CommandList className="max-h-64">
            <CommandEmpty>No time like that.</CommandEmpty>
            <CommandGroup>
              {times.map((time) => (
                <CommandItem
                  key={time}
                  value={searchValue(time)}
                  className="tabular-nums"
                  onSelect={() => {
                    onChange(time);
                    setOpen(false);
                  }}
                >
                  {formatClock(time, hour12)}
                  {time === value ? <Check className="size-4" /> : null}
                  {after === undefined ? null : (
                    <span className="ml-auto text-muted-foreground">
                      {lengthLabel(toMinutes(time) - floor)}
                    </span>
                  )}
                </CommandItem>
              ))}
            </CommandGroup>
          </CommandList>
        </Command>
      </PopoverContent>
    </Popover>
  );
}
