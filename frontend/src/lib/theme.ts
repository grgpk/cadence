import { useEffect, useState } from "react";

const STORAGE_KEY = "cadence-theme";
const DARK_CLASS = "dark";

export const THEMES = { LIGHT: "light", DARK: "dark" } as const;
type Theme = (typeof THEMES)[keyof typeof THEMES];

/** Runs in <head> before paint so the stored or system theme never flashes. */
export const THEME_INIT_SCRIPT = `(function(){try{var t=localStorage.getItem("${STORAGE_KEY}");var d=t?t==="${THEMES.DARK}":matchMedia("(prefers-color-scheme: dark)").matches;document.documentElement.classList.toggle("${DARK_CLASS}",d)}catch(e){}})()`;

export function useTheme() {
  // null until mounted: the server cannot know the viewer's theme.
  const [theme, setTheme] = useState<Theme | null>(null);

  useEffect(() => {
    setTheme(
      document.documentElement.classList.contains(DARK_CLASS)
        ? THEMES.DARK
        : THEMES.LIGHT,
    );
  }, []);

  function toggleTheme() {
    const next = theme === THEMES.DARK ? THEMES.LIGHT : THEMES.DARK;
    document.documentElement.classList.toggle(DARK_CLASS, next === THEMES.DARK);
    try {
      localStorage.setItem(STORAGE_KEY, next);
    } catch {
      // Private windows can block storage; the choice then lasts for this page.
    }
    setTheme(next);
  }

  return { theme, toggleTheme };
}
