import { cn } from "cn";
import { Check } from "lucide-react";
import { useEffect, useState, type CSSProperties } from "react";

import { Input } from "../../components/ui/input";
import { Label } from "../../components/ui/label";
import { INK, isHexColor, readableOn } from "../../lib/color";
import { PlanNotice } from "../billing/plan-notice";

/** Starting points that read well on a white page. */
const PRESETS = [
  { name: "Ink", hex: INK },
  { name: "Violet", hex: "#7c3aed" },
  { name: "Teal", hex: "#0f766e" },
  { name: "Green", hex: "#15803d" },
  { name: "Rose", hex: "#be123c" },
  { name: "Amber", hex: "#b45309" },
] as const;

/** The booking page's colour for buttons and the picked time. Pro. */
export function AccentField({
  id,
  value,
  locked,
  onChange,
}: {
  id: string;
  value: string;
  /** The plan does not include it: shown, not changeable. */
  locked: boolean;
  onChange: (hex: string) => void;
}) {
  // The text box may hold a half-typed colour; only a whole one is passed on.
  const [typed, setTyped] = useState(value);
  useEffect(() => setTyped(value), [value]);
  const preview = {
    backgroundColor: value,
    color: readableOn(value),
  } satisfies CSSProperties;

  return (
    <div className="grid gap-3">
      <div>
        <Label htmlFor={`${id}-hex`}>Accent colour</Label>
        <p className="mt-1 text-sm text-muted-foreground">
          Buttons and the picked time on your booking page.
        </p>
      </div>
      <fieldset disabled={locked} className="flex flex-wrap items-center gap-3">
        <legend className="sr-only">Accent colour presets</legend>
        <div className="flex gap-1.5">
          {PRESETS.map((preset) => (
            <button
              key={preset.hex}
              type="button"
              aria-label={preset.name}
              aria-pressed={value === preset.hex}
              onClick={() => onChange(preset.hex)}
              style={{ backgroundColor: preset.hex }}
              className="grid size-8 place-items-center rounded-full ring-offset-2 ring-offset-background outline-none focus-visible:ring-2 focus-visible:ring-ring disabled:opacity-50 aria-pressed:ring-2 aria-pressed:ring-foreground"
            >
              {value === preset.hex ? (
                <Check className="size-4" style={{ color: readableOn(preset.hex) }} />
              ) : null}
            </button>
          ))}
        </div>
        <div className="flex items-center gap-2">
          <input
            type="color"
            aria-label="Pick any colour"
            value={value}
            onChange={(event) => onChange(event.target.value)}
            className="size-8 cursor-pointer rounded-md border bg-transparent p-0.5 disabled:cursor-not-allowed"
          />
          <Input
            id={`${id}-hex`}
            value={typed}
            maxLength={7}
            spellCheck={false}
            className={cn("w-28 font-medium", !isHexColor(typed) && "border-destructive")}
            onChange={(event) => {
              const next = event.target.value.trim();
              setTyped(next);
              if (isHexColor(next)) onChange(next.toLowerCase());
            }}
          />
        </div>
        <span
          aria-hidden="true"
          className="rounded-md px-3 py-1.5 text-sm font-medium"
          style={preview}
        >
          Confirm booking
        </span>
      </fieldset>
      {locked ? (
        <PlanNotice>
          Your own accent colour comes with Pro. A saved one stays saved.
        </PlanNotice>
      ) : null}
    </div>
  );
}
