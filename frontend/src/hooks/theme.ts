import { useEffect, useState } from "react";

const STORAGE_KEY = "cadence-theme";
const DARK_CLASS = "dark";
const SWITCH_ATTRIBUTE = "data-theme-switch";

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

  /** Pass where the click landed and the new theme opens as a circle from there. */
  function toggleTheme(origin?: { x: number; y: number }) {
    const next = theme === THEMES.DARK ? THEMES.LIGHT : THEMES.DARK;
    const root = document.documentElement;
    const apply = () => root.classList.toggle(DARK_CLASS, next === THEMES.DARK);
    if (
      origin &&
      "startViewTransition" in document &&
      !matchMedia("(prefers-reduced-motion: reduce)").matches
    ) {
      const radius = Math.hypot(
        Math.max(origin.x, innerWidth - origin.x),
        Math.max(origin.y, innerHeight - origin.y),
      );
      root.style.setProperty("--reveal-x", `${origin.x}px`);
      root.style.setProperty("--reveal-y", `${origin.y}px`);
      root.style.setProperty("--reveal-r", `${radius}px`);
      root.setAttribute(SWITCH_ATTRIBUTE, "");
      void document
        .startViewTransition(apply)
        .finished.finally(() => root.removeAttribute(SWITCH_ATTRIBUTE));
    } else {
      apply();
    }
    try {
      localStorage.setItem(STORAGE_KEY, next);
    } catch {
      // Private windows can block storage; the choice then lasts for this page.
    }
    setTheme(next);
  }

  return { theme, toggleTheme };
}
