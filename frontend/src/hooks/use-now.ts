import { useEffect, useState } from "react";

const MINUTE_MS = 60_000;

/** The current time, refreshed each minute. `null` until the browser has mounted. */
export function useNow(): Date | null {
  const [now, setNow] = useState<Date | null>(null);
  useEffect(() => {
    setNow(new Date());
    const timer = window.setInterval(() => setNow(new Date()), MINUTE_MS);
    return () => window.clearInterval(timer);
  }, []);
  return now;
}
