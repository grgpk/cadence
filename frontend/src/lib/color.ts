/** The ink the app writes on light surfaces. */
export const INK = "#111111";
const WHITE = "#ffffff";

/** Relative luminance of `#rrggbb`, as WCAG defines it. */
function luminance(hex: string): number {
  const channels = [1, 3, 5].map((start) => {
    const value = Number.parseInt(hex.slice(start, start + 2), 16) / 255;
    return value <= 0.03928 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4;
  });
  const [red = 0, green = 0, blue = 0] = channels;
  return 0.2126 * red + 0.7152 * green + 0.0722 * blue;
}

function contrast(a: string, b: string): number {
  const [light, dark] = [luminance(a), luminance(b)].sort((x, y) => y - x) as [
    number,
    number,
  ];
  return (light + 0.05) / (dark + 0.05);
}

/** The dark theme's page, `.dark --background` (oklch 0.185 0.006 280) in globals.css. */
const DARK_PAGE = "#121215";
/** WCAG's least contrast for buttons, focus rings and other UI parts. */
const UI_CONTRAST = 3;

/** `hex` moved `share` (0 to 1) of the way to white. */
function towardsWhite(hex: string, share: number): string {
  const channels = [1, 3, 5].map((start) => {
    const value = Number.parseInt(hex.slice(start, start + 2), 16);
    return Math.round(value + (255 - value) * share)
      .toString(16)
      .padStart(2, "0");
  });
  return `#${channels.join("")}`;
}

/**
 * The host's colour as a dark page shows it: as picked when it stands out
 * from the page, otherwise lightened in steps until it does.
 */
export function forDarkPage(hex: string): string {
  for (let step = 0; step <= 10; step += 1) {
    const shown = towardsWhite(hex, step / 10);
    if (contrast(shown, DARK_PAGE) >= UI_CONTRAST) return shown;
  }
  return WHITE;
}

/** White or ink, whichever reads better on `hex`. */
export function readableOn(hex: string): string {
  return contrast(hex, WHITE) >= contrast(hex, INK) ? WHITE : INK;
}

/** Whether `value` is a colour the booking page accepts. */
export function isHexColor(value: string): boolean {
  return /^#[0-9a-f]{6}$/i.test(value);
}
