import { useSyncExternalStore } from "react";

const STORAGE_KEY = "cadence-theme";
const DARK_CLASS = "dark";
const SWITCH_ATTRIBUTE = "data-theme-switch";
const DARK_QUERY = "(prefers-color-scheme: dark)";

export const THEMES = { LIGHT: "light", DARK: "dark" } as const;
type Theme = (typeof THEMES)[keyof typeof THEMES];

export const THEME_PREFERENCES = { SYSTEM: "system", ...THEMES } as const;
export type ThemePreference = (typeof THEME_PREFERENCES)[keyof typeof THEME_PREFERENCES];
/** What a device that never chose gets. */
export const DEFAULT_THEME_PREFERENCE: ThemePreference = THEME_PREFERENCES.DARK;

/** Runs in <head> before paint so the stored or default theme never flashes. */
export const THEME_INIT_SCRIPT = `(function(){try{var t=localStorage.getItem("${STORAGE_KEY}");var d=t==="${THEME_PREFERENCES.LIGHT}"?false:t==="${THEME_PREFERENCES.SYSTEM}"?matchMedia("${DARK_QUERY}").matches:true;document.documentElement.classList.toggle("${DARK_CLASS}",d)}catch(e){}})()`;

export function resolveTheme(preference: ThemePreference, systemDark: boolean): Theme {
  if (preference !== THEME_PREFERENCES.SYSTEM) return preference;
  return systemDark ? THEMES.DARK : THEMES.LIGHT;
}

function isPreference(value: string | null): value is ThemePreference {
  return Object.values(THEME_PREFERENCES).some((preference) => preference === value);
}

function readPreference(): ThemePreference {
  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (isPreference(stored)) return stored;
  } catch {
    // Private windows can block storage.
  }
  return DEFAULT_THEME_PREFERENCE;
}

function isDark() {
  return document.documentElement.classList.contains(DARK_CLASS);
}

// Every useTheme on the page reads one store, so the top bar toggle and the
// settings card never disagree.
const listeners = new Set<() => void>();

function notify() {
  for (const listener of listeners) listener();
}

/** Another tab chose a theme, or the system flipped while it is followed. */
function onOutsideChange(event: Event) {
  if (event instanceof StorageEvent && event.key !== STORAGE_KEY) return;
  document.documentElement.classList.toggle(
    DARK_CLASS,
    resolveTheme(readPreference(), matchMedia(DARK_QUERY).matches) === THEMES.DARK,
  );
  notify();
}

function subscribe(listener: () => void) {
  if (listeners.size === 0) {
    window.addEventListener("storage", onOutsideChange);
    matchMedia(DARK_QUERY).addEventListener("change", onOutsideChange);
  }
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
    if (listeners.size > 0) return;
    window.removeEventListener("storage", onOutsideChange);
    matchMedia(DARK_QUERY).removeEventListener("change", onOutsideChange);
  };
}

const serverSnapshot = () => null;

/** Pass where the click landed and the new theme opens as a circle from there. */
function setThemePreference(
  preference: ThemePreference,
  origin?: { x: number; y: number },
) {
  const dark = resolveTheme(preference, matchMedia(DARK_QUERY).matches) === THEMES.DARK;
  const root = document.documentElement;
  const apply = () => {
    root.classList.toggle(DARK_CLASS, dark);
    notify();
  };
  if (
    origin &&
    dark !== isDark() &&
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
    localStorage.setItem(STORAGE_KEY, preference);
  } catch {
    // Private windows can block storage; the choice then lasts for this page.
  }
  notify();
}

export function useTheme() {
  // null until mounted: the server cannot know the viewer's theme.
  const preference = useSyncExternalStore(subscribe, readPreference, serverSnapshot);
  const dark = useSyncExternalStore(subscribe, isDark, serverSnapshot);
  const theme: Theme | null = dark === null ? null : dark ? THEMES.DARK : THEMES.LIGHT;

  function toggleTheme(origin?: { x: number; y: number }) {
    setThemePreference(theme === THEMES.DARK ? THEMES.LIGHT : THEMES.DARK, origin);
  }

  return { theme, preference, setPreference: setThemePreference, toggleTheme };
}
