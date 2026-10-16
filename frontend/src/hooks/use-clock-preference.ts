import { useCallback, useSyncExternalStore } from "react";

import { prefersHour12 } from "../lib/format";

const STORAGE_KEY = "cadence-hour12";

/** The two clocks a visitor can choose between. */
export const CLOCK_OPTIONS = [
  { value: true, label: "12h" },
  { value: false, label: "24h" },
] as const;

/** A choice made this visit, kept even when storage is blocked. */
let chosen: boolean | null = null;
const listeners = new Set<() => void>();

function read(): boolean {
  if (chosen !== null) return chosen;
  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (stored !== null) return stored === "true";
  } catch {
    // Storage can be blocked; fall back to the device's own style.
  }
  return prefersHour12();
}

function subscribe(listener: () => void): () => void {
  listeners.add(listener);
  // Another tab changed it.
  window.addEventListener("storage", listener);
  return () => {
    listeners.delete(listener);
    window.removeEventListener("storage", listener);
  };
}

/**
 * Whether the visitor reads times as "2:00 PM" or "14:00": what they chose on
 * any page, remembered on this device, else the device's own style. On the
 * server, which knows neither, it is 24h until the page is in the browser.
 */
export function useClockPreference(): [boolean, (hour12: boolean) => void] {
  const hour12 = useSyncExternalStore(subscribe, read, () => false);
  const setHour12 = useCallback((next: boolean) => {
    chosen = next;
    try {
      localStorage.setItem(STORAGE_KEY, String(next));
    } catch {
      // The choice still holds for this visit.
    }
    for (const listener of listeners) listener();
  }, []);
  return [hour12, setHour12];
}
