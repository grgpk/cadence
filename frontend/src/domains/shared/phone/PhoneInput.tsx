import { ChevronsUpDown, X } from "lucide-react";
import * as React from "react";

import {
  Command,
  CommandEmpty,
  CommandGroup,
  CommandInput,
  CommandItem,
  CommandList,
} from "../../../components/ui/command";
import { Input } from "../../../components/ui/input";
import { Popover, PopoverContent, PopoverTrigger } from "../../../components/ui/popover";
import { COMMON_COUNTRIES, COUNTRIES, dialCodeFormatted, type Country } from "./country";
import {
  digitsOnly,
  formatForCountry,
  formatPhone,
  phonePlaceholder,
} from "./phone-format";

const REST_OF_COUNTRIES = COUNTRIES.filter(
  (country) => !COMMON_COUNTRIES.some((common) => common.variant === country.variant),
);

function CountryItem({
  country,
  selectedCountry,
  onSelect,
}: {
  country: Country;
  selectedCountry: Country | null;
  onSelect: (country: Country) => void;
}) {
  const searchValue = `${country.name} ${country.alpha2} ${dialCodeFormatted(country)}`;

  return (
    <CommandItem value={searchValue} onSelect={() => onSelect(country)}>
      <span className="text-base">{country.flag}</span>
      <span className="flex-1 truncate">{country.name}</span>
      <span className="w-12 text-right text-muted-foreground">
        {dialCodeFormatted(country)}
      </span>
      {selectedCountry?.variant === country.variant && <span className="text-xs">✓</span>}
    </CommandItem>
  );
}

type PhoneInputProps = {
  country: Country;
  digits: string;
  onCountryChange: (country: Country) => void;
  onDigitsChange: (digits: string) => void;
  invalid?: boolean;
  disabled?: boolean;
  onBlur?: () => void;
};

export function PhoneInput({
  country,
  digits,
  onCountryChange,
  onDigitsChange,
  invalid = false,
  disabled = false,
  onBlur,
}: PhoneInputProps) {
  const [open, setOpen] = React.useState(false);

  return (
    <div className="flex w-full">
      <Popover open={open} onOpenChange={setOpen}>
        <PopoverTrigger asChild>
          <button
            type="button"
            disabled={disabled}
            aria-label="Select country"
            className="flex h-9 items-center gap-1 rounded-l-md rounded-r-none border border-r-0 border-input bg-transparent px-3 shadow-xs outline-none focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/50 disabled:cursor-not-allowed disabled:opacity-50"
          >
            <span className="text-base">{country.flag}</span>
            <span className="text-xs text-muted-foreground">
              {dialCodeFormatted(country)}
            </span>
            <ChevronsUpDown className="ml-1 size-3 opacity-50" />
          </button>
        </PopoverTrigger>

        <PopoverContent align="start" className="w-[280px] p-0">
          <Command>
            <CommandInput placeholder="Search country..." />
            <CommandList className="max-h-[280px] min-h-0">
              <CommandEmpty>No country found.</CommandEmpty>
              <CommandGroup>
                {COMMON_COUNTRIES.map((commonCountry) => (
                  <CountryItem
                    key={commonCountry.variant}
                    country={commonCountry}
                    selectedCountry={country}
                    onSelect={(selected) => {
                      onCountryChange(selected);
                      setOpen(false);
                    }}
                  />
                ))}
              </CommandGroup>
              <CommandGroup heading="All countries">
                {REST_OF_COUNTRIES.map((restCountry) => (
                  <CountryItem
                    key={restCountry.variant}
                    country={restCountry}
                    selectedCountry={country}
                    onSelect={(selected) => {
                      onCountryChange(selected);
                      setOpen(false);
                    }}
                  />
                ))}
              </CommandGroup>
            </CommandList>
          </Command>
        </PopoverContent>
      </Popover>

      <div className="relative flex-1">
        <Input
          className="w-full rounded-l-none pr-8"
          type="tel"
          inputMode="numeric"
          autoComplete="tel-national"
          placeholder={phonePlaceholder(country.variant)}
          disabled={disabled}
          aria-label="Phone number"
          aria-invalid={invalid}
          value={formatPhone(country.variant, digits)}
          onChange={(event) =>
            onDigitsChange(
              digitsOnly(event.target.value, formatForCountry(country.variant).maxDigits),
            )
          }
          onBlur={onBlur}
        />
        {digits.length > 0 && !disabled && (
          <button
            type="button"
            tabIndex={-1}
            aria-label="Clear phone number"
            className="absolute top-1/2 right-2 -translate-y-1/2 rounded-sm p-0.5 text-muted-foreground transition-colors hover:bg-muted hover:text-foreground"
            onClick={() => onDigitsChange("")}
          >
            <X className="size-4" />
          </button>
        )}
      </div>
    </div>
  );
}
