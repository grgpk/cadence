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

/** White or ink, whichever reads better on `hex`. */
export function readableOn(hex: string): string {
  return contrast(hex, WHITE) >= contrast(hex, INK) ? WHITE : INK;
}

/** Whether `value` is a colour the booking page accepts. */
export function isHexColor(value: string): boolean {
  return /^#[0-9a-f]{6}$/i.test(value);
}
