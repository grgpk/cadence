import { Monitor, Moon, Sun, type LucideIcon } from "lucide-react";

import { Label } from "../../components/ui/label";
import { RadioGroup, RadioGroupItem } from "../../components/ui/radio-group";
import {
  DEFAULT_THEME_PREFERENCE,
  THEME_PREFERENCES,
  THEMES,
  useTheme,
  type ThemePreference,
} from "../../hooks/theme";

const ID = "appearance";

const OPTIONS: { value: ThemePreference; label: string; icon: LucideIcon }[] = [
  { value: THEME_PREFERENCES.SYSTEM, label: "System", icon: Monitor },
  { value: THEME_PREFERENCES.LIGHT, label: "Light", icon: Sun },
  { value: THEME_PREFERENCES.DARK, label: "Dark", icon: Moon },
];

type Theme = (typeof THEMES)[keyof typeof THEMES];

// `dark:` would follow the page's theme, not the preview's, so each preview
// names its own tape fills (the same ones `OPEN_STYLES` uses per theme).
const TAPE_FILLS: Record<Theme, { dawn: string; noon: string }> = {
  [THEMES.LIGHT]: { dawn: "bg-dawn/35", noon: "bg-noon/30" },
  [THEMES.DARK]: { dawn: "bg-dawn/10", noon: "bg-noon/10" },
};

/** The overview page in miniature, drawn with the real tokens of one theme. */
function Preview({ theme, className = "" }: { theme: Theme; className?: string }) {
  const fills = TAPE_FILLS[theme];
  return (
    <div className={`${theme} absolute inset-0 flex bg-background ${className}`}>
      <div className="flex w-[22%] flex-col gap-1 border-r bg-sidebar p-1.5">
        <span className="mb-1 flex h-2 items-end gap-px">
          <span className="h-1/2 w-0.5 rounded-full bg-foreground" />
          <span className="h-full w-0.5 rounded-full bg-signal" />
          <span className="h-3/4 w-0.5 rounded-full bg-foreground" />
        </span>
        <span className="h-1.5 rounded-sm bg-accent" />
        <span className="h-1 w-3/4 rounded-full bg-foreground/15" />
        <span className="h-1 w-2/3 rounded-full bg-foreground/15" />
      </div>
      <div className="flex flex-1 flex-col gap-1.5 p-2">
        <span className="h-1.5 w-1/2 rounded-full bg-foreground/80" />
        <div className="relative flex h-6 overflow-hidden rounded-[3px] border bg-card p-1">
          <span className={`w-2/5 border-y border-dawn ${fills.dawn}`} />
          <span className={`flex-1 border-y border-noon ${fills.noon}`} />
          <span className="absolute inset-y-0 left-[30%] w-px bg-signal" />
        </div>
        {["w-1/2", "w-1/3"].map((width) => (
          <div
            key={width}
            className="flex items-center gap-1 rounded-[3px] border bg-card px-1 py-1"
          >
            <span className="h-2.5 w-0.5 rounded-full bg-noon" />
            <span className="h-1 w-3 rounded-full bg-foreground/70" />
            <span className={`h-1 rounded-full bg-muted-foreground/40 ${width}`} />
          </div>
        ))}
      </div>
    </div>
  );
}

/** How the app looks on this device. Saved on its own, in the browser. */
export function AppearanceSection() {
  const { preference, setPreference } = useTheme();

  return (
    <RadioGroup
      aria-label="Theme"
      // Nothing is checked until mounted: the server cannot know the choice.
      value={preference ?? ""}
      onValueChange={(value) => {
        const option = OPTIONS.find((candidate) => candidate.value === value);
        if (!option) return;
        // The chosen radio's center, so a keyboard choice opens from the same place.
        const box = document.getElementById(`${ID}-${value}`)?.getBoundingClientRect();
        setPreference(
          option.value,
          box && { x: box.left + box.width / 2, y: box.top + box.height / 2 },
        );
      }}
      className="grid-cols-3"
    >
      {OPTIONS.map(({ value, label, icon: Icon }) => (
        <Label
          key={value}
          htmlFor={`${ID}-${value}`}
          className="group grid cursor-pointer gap-2.5 font-normal"
        >
          <div
            aria-hidden="true"
            className="relative aspect-[4/3] overflow-hidden rounded-lg border transition-[border-color,box-shadow] group-hover:border-muted-foreground/50 group-has-[[data-state=checked]]:border-primary group-has-[[data-state=checked]]:ring-[3px] group-has-[[data-state=checked]]:ring-ring/30"
          >
            {value === THEME_PREFERENCES.SYSTEM ? (
              <>
                <Preview theme={THEMES.LIGHT} />
                {/* Night falls across the same page. */}
                <Preview
                  theme={THEMES.DARK}
                  className="[clip-path:polygon(100%_0,100%_100%,0_100%)]"
                />
              </>
            ) : (
              <Preview theme={value} />
            )}
          </div>
          <span className="flex flex-wrap items-center gap-2">
            <RadioGroupItem id={`${ID}-${value}`} value={value} />
            <Icon className="size-4 text-muted-foreground" />
            {label}
            {value === DEFAULT_THEME_PREFERENCE ? (
              <span className="hidden text-xs text-muted-foreground sm:inline">
                Default
              </span>
            ) : null}
          </span>
        </Label>
      ))}
    </RadioGroup>
  );
}
