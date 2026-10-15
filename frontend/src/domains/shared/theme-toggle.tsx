import { Moon, Sun } from "lucide-react";

import { Button } from "../../components/ui/button";
import { THEMES, useTheme } from "../../lib/theme";

export function ThemeToggle() {
  const { theme, toggleTheme } = useTheme();
  const isDark = theme === THEMES.DARK;

  return (
    <Button
      type="button"
      variant="ghost"
      size="icon-sm"
      aria-label={isDark ? "Switch to light theme" : "Switch to dark theme"}
      onClick={(event) => {
        // The button's center, so a keyboard press opens from the same place.
        const box = event.currentTarget.getBoundingClientRect();
        toggleTheme({ x: box.left + box.width / 2, y: box.top + box.height / 2 });
      }}
    >
      {isDark ? <Sun /> : <Moon />}
    </Button>
  );
}
