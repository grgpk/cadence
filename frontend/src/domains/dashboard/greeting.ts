import { dayPhase, type DayPhase, zonedTime } from "../../lib/zoned-time";

const SALUTES: Record<DayPhase, string> = {
  Morning: "Good morning",
  Afternoon: "Good afternoon",
  Evening: "Good evening",
};

/** "Good afternoon, Giorgi" on the host's own clock, and the part of the day it names. */
export function greeting(
  firstName: string | undefined,
  now: Date,
  timeZone: string,
): { text: string; phase: DayPhase } {
  const phase = dayPhase(zonedTime(now, timeZone).hour);
  const salute = SALUTES[phase];
  return { text: firstName ? `${salute}, ${firstName}` : salute, phase };
}
