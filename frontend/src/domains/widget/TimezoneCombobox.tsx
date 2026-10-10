import { Check, ChevronDown } from "lucide-react";
import { useState } from "react";

import {
  Command,
  CommandEmpty,
  CommandGroup,
  CommandInput,
  CommandItem,
  CommandList,
} from "../../components/ui/command";
import { Popover, PopoverContent, PopoverTrigger } from "../../components/ui/popover";
import {
  ALL_COUNTRY_TIMEZONES,
  timezoneDisplayWithTime,
  timezoneFlag,
  type CountryTimezone,
} from "../shared/phone/country";

export function TimezoneCombobox({
  id,
  selected,
  fallbackLabel = "Choose a timezone",
  hour12,
  onSelect,
}: {
  id?: string;
  /** Missing when the stored timezone is not in the country list, such as UTC. */
  selected: CountryTimezone | undefined;
  /** Shown instead when nothing in the list is selected. */
  fallbackLabel?: string;
  /** The visitor's choice of clock, where there is one. Otherwise the device's. */
  hour12?: boolean;
  onSelect: (tz: CountryTimezone) => void;
}) {
  const [open, setOpen] = useState(false);
  const label = selected ? timezoneDisplayWithTime(selected, hour12) : fallbackLabel;
  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger asChild>
        <button
          id={id}
          type="button"
          aria-label={`Timezone: ${label}. Change timezone`}
          className="flex min-w-0 items-center gap-1 rounded-sm text-left text-sm font-medium outline-none hover:underline focus-visible:ring-[3px] focus-visible:ring-ring/50"
        >
          <span className="min-w-0 truncate">{label}</span>
          <ChevronDown className="size-4 shrink-0 text-muted-foreground" />
        </button>
      </PopoverTrigger>

      <PopoverContent align="start" className="w-[min(320px,calc(100vw-2rem))] p-0">
        <Command>
          <CommandInput
            aria-label="Search country or timezone"
            placeholder="Search country or timezone..."
          />
          <CommandList className="min-h-0">
            <CommandEmpty>No timezone found.</CommandEmpty>
            <CommandGroup>
              {ALL_COUNTRY_TIMEZONES.map((tz) => (
                <CommandItem
                  key={tz.iana}
                  value={timezoneDisplayWithTime(tz, hour12)}
                  onSelect={() => {
                    onSelect(tz);
                    setOpen(false);
                  }}
                >
                  <span className="text-base">{timezoneFlag(tz)}</span>
                  {timezoneDisplayWithTime(tz, hour12)}
                  {selected?.iana === tz.iana && <Check className="ml-auto size-4" />}
                </CommandItem>
              ))}
            </CommandGroup>
          </CommandList>
        </Command>
      </PopoverContent>
    </Popover>
  );
}
