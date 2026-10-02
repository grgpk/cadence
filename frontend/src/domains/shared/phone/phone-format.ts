// Mirrors app_crates/app_types/src/phone_number.rs (PhoneNumber + PhoneFormat).

import { PHONE_FORMATS, type PhoneFormat } from "./generated/phone-format";

const DEFAULT_FORMAT: PhoneFormat = { groups: [3, 3, 4], maxDigits: 10 };

export function formatForCountry(variant: string): PhoneFormat {
  return PHONE_FORMATS[variant] ?? DEFAULT_FORMAT;
}

export function digitsOnly(input: string, maxDigits: number): string {
  return input.replace(/\D/g, "").slice(0, maxDigits);
}

function isGermanyMobileFormat(format: PhoneFormat): boolean {
  return (
    format.maxDigits === 11 &&
    format.groups.length === 3 &&
    format.groups[0] === 3 &&
    format.groups[1] === 3 &&
    format.groups[2] === 4
  );
}

function germanyMobilePrefixLen(digits: string): number {
  if (digits.startsWith("1609") || digits.startsWith("15")) return 4;
  return 3;
}

function formatGermanyMobile(digits: string): string {
  if (!digits) return "";
  const prefixLen = Math.min(germanyMobilePrefixLen(digits), digits.length);
  const prefix = digits.slice(0, prefixLen);
  const rest = digits.slice(prefixLen);
  return rest ? `${prefix} ${rest}` : prefix;
}

function formatWithGroups(groups: number[], digits: string): string {
  let result = "";
  let index = 0;
  for (let i = 0; i < groups.length; i++) {
    if (index >= digits.length) break;
    if (i > 0) result += " ";
    result += digits.slice(index, index + groups[i]);
    index += groups[i];
  }
  result += digits.slice(index);
  return result;
}

/** Mirrors PhoneNumber::format (variant param is the Country enum variant, e.g. "Germany"). */
export function formatPhone(variant: string, digits: string): string {
  const format = formatForCountry(variant);
  if (isGermanyMobileFormat(format)) return formatGermanyMobile(digits);
  return formatWithGroups(format.groups, digits);
}

export function phonePlaceholder(variant: string): string {
  const format = formatForCountry(variant);
  if (isGermanyMobileFormat(format)) return "170 1234567";
  const digits = Array.from({ length: format.maxDigits }, (_, i) => String(i % 10)).join(
    "",
  );
  return formatPhone(variant, digits);
}

export const MIN_REPEATED_DIGITS = 6;

/** Mirrors PhoneNumber::has_repeated_digits. */
export function hasRepeatedDigits(digits: string, minRepeated: number): boolean {
  if (minRepeated === 0 || digits.length < minRepeated) return false;
  let count = 1;
  for (let i = 1; i < digits.length; i++) {
    if (digits[i] === digits[i - 1]) {
      count += 1;
      if (count >= minRepeated) return true;
    } else {
      count = 1;
    }
  }
  return false;
}
