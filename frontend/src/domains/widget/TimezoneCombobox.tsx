"use client";

import { ChevronDown, Globe, Search } from "lucide-react";

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
  selected,
  onSelect,
}: {
  selected: CountryTimezone;
  onSelect: (tz: CountryTimezone) => void;
}) {
  return (
    <div className="flex items-center gap-2 px-0.5 py-1">
      <Globe className="size-5 shrink-0 text-muted-foreground" />
      <Popover>
        <PopoverTrigger asChild>
          <button
            type="button"
            className="flex w-fit items-center gap-1 border-none bg-transparent p-0 shadow-none outline-none"
          >
            <span className="min-w-0 truncate text-left text-sm font-medium whitespace-nowrap">
              {timezoneDisplayWithTime(selected)}
            </span>
            <ChevronDown className="size-4 shrink-0 text-muted-foreground" />
          </button>
        </PopoverTrigger>

        <PopoverContent align="start" className="w-[320px] p-0">
          <Command>
            <div className="flex items-center gap-2 border-b px-2">
              <Search className="size-4 shrink-0 text-muted-foreground" />
              <CommandInput placeholder="Search country or timezone..." />
            </div>
            <CommandList className="min-h-0">
              <CommandEmpty>No timezone found.</CommandEmpty>
              <CommandGroup>
                {ALL_COUNTRY_TIMEZONES.map((tz) => (
                  <CommandItem
                    key={tz.iana}
                    value={timezoneDisplayWithTime(tz)}
                    onSelect={() => onSelect(tz)}
                  >
                    <span className="text-base">{timezoneFlag(tz)}</span>
                    {timezoneDisplayWithTime(tz)}
                    {selected.iana === tz.iana && (
                      <span className="ml-auto text-xs">✓</span>
                    )}
                  </CommandItem>
                ))}
              </CommandGroup>
            </CommandList>
          </Command>
        </PopoverContent>
      </Popover>
    </div>
  );
}
