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
      onClick={toggleTheme}
    >
      {isDark ? <Sun /> : <Moon />}
    </Button>
  );
}
