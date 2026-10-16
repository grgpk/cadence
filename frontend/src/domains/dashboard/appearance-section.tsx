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

/** A tiny app window drawn with the real tokens of one theme. */
function Preview({ theme }: { theme: (typeof THEMES)[keyof typeof THEMES] }) {
  return (
    <div className={`${theme} flex h-full flex-1 gap-1.5 bg-background p-2`}>
      <div className="w-1/4 rounded-sm bg-sidebar" />
      <div className="grid flex-1 content-start gap-1.5 rounded-sm bg-card p-1.5">
        <div className="h-1.5 w-3/4 rounded-full bg-foreground/70" />
        <div className="h-1.5 w-1/2 rounded-full bg-muted-foreground/50" />
        <div className="mt-1 h-3 w-1/3 rounded-sm bg-dusk" />
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
            className="flex aspect-[4/3] overflow-hidden rounded-lg border transition-[border-color,box-shadow] group-has-[[data-state=checked]]:border-primary group-has-[[data-state=checked]]:ring-[3px] group-has-[[data-state=checked]]:ring-ring/30"
          >
            {value === THEME_PREFERENCES.SYSTEM ? (
              <>
                <Preview theme={THEMES.LIGHT} />
                <Preview theme={THEMES.DARK} />
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
              <span className="text-xs text-muted-foreground">Default</span>
            ) : null}
          </span>
        </Label>
      ))}
    </RadioGroup>
  );
}
