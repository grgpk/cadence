import { formatTime } from "../../../lib/format";
import { COUNTRIES, type Country } from "./generated/country-data";
import { ALL_COUNTRY_TIMEZONES, type CountryTimezone } from "./generated/timezone-data";

export type { Country, CountryTimezone };

const byVariant = new Map(COUNTRIES.map((country) => [country.variant, country]));
const byAlpha2 = new Map(COUNTRIES.map((country) => [country.alpha2, country]));

/** Where a visitor is taken to be when neither the request nor the browser says. */
export const DEFAULT_TIMEZONE: CountryTimezone = {
  variant: "France",
  region: null,
  iana: "Europe/Paris",
};

export function getCountryByVariant(variant: string): Country | undefined {
  return byVariant.get(variant);
}

export function getCountryByAlpha2(alpha2: string): Country | undefined {
  return byAlpha2.get(alpha2.toUpperCase());
}

const COMMON_COUNTRY_VARIANTS = [
  "UnitedStatesOfAmerica",
  "UnitedKingdom",
  "France",
  "Germany",
  "Canada",
  "Australia",
  "Spain",
  "Italy",
  "Japan",
  "China",
  "India",
  "Brazil",
  "Mexico",
] as const;

export const COMMON_COUNTRIES: Country[] = COMMON_COUNTRY_VARIANTS.map((variant) =>
  byVariant.get(variant),
).filter((country): country is Country => country !== undefined);

export function dialCodeFormatted(country: Country): string {
  return `+${country.dialCode}`;
}

function timezoneLabel(timezone: CountryTimezone): string {
  const country = getCountryByVariant(timezone.variant);
  const countryName = country?.name ?? timezone.variant;
  return timezone.region ? `${countryName} - ${timezone.region}` : countryName;
}

export function timezoneFlag(timezone: CountryTimezone): string {
  return getCountryByVariant(timezone.variant)?.flag ?? "🌐";
}

/** The zone with its current time, in the device's 12 or 24 hour style unless told. */
export function timezoneDisplayWithTime(
  timezone: CountryTimezone,
  hour12?: boolean,
): string {
  const time = formatTime(new Date().toISOString(), timezone.iana, hour12);
  return `${timezoneLabel(timezone)} - ${time}`;
}

export function defaultTimezoneForCountry(variant: string): CountryTimezone | undefined {
  return ALL_COUNTRY_TIMEZONES.find((timezone) => timezone.variant === variant);
}

export function detectBrowserCountryTimezone(): CountryTimezone | undefined {
  try {
    const iana = Intl.DateTimeFormat().resolvedOptions().timeZone;
    if (!iana) return undefined;
    return ALL_COUNTRY_TIMEZONES.find((timezone) => timezone.iana === iana);
  } catch {
    return undefined;
  }
}

export { ALL_COUNTRY_TIMEZONES, COUNTRIES };
