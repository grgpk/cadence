import { cn } from "cn";
import { Check, Globe, Link2 } from "lucide-react";
import {
  type CSSProperties,
  type RefObject,
  useCallback,
  useEffect,
  useLayoutEffect,
  useRef,
  useState,
} from "react";

import { useOnScreen } from "../../hooks/use-on-screen";
import { monthLabel, monthWeeks } from "../../lib/civil-day";
import { APP_NAME } from "../../lib/config";
import { dayPhase } from "../../lib/zoned-time";
import { GoogleMeetIcon } from "../integrations/google-meet-icon";
import { PHASE_STYLES } from "../shared/day-phase-styles";
import { OPEN_STYLES } from "./day-tape";

/** The scene, one beat at a time. The last beat is the still frame. */
const STEP = { IDLE: 0, DAY: 1, TIME: 2, CONFIRM: 3, FLY: 4, BOOKED: 5 } as const;
type Step = (typeof STEP)[keyof typeof STEP];
const LAST_STEP: Step = STEP.BOOKED;
/** How long each beat holds before the next, in milliseconds. */
const BEAT_MS: Record<Step, number> = {
  [STEP.IDLE]: 1100,
  [STEP.DAY]: 1200,
  [STEP.TIME]: 1000,
  [STEP.CONFIRM]: 800,
  [STEP.FLY]: 900,
  [STEP.BOOKED]: 3200,
};
/** Matches the cursor's glide, so the click lands when the cursor does. */
const GLIDE_MS = 650;

const YEAR = 2026;
const MONTH = 11;
const PICKED_DAY = 10;
const WEEKDAYS = ["Mo", "Tu", "We", "Th", "Fr", "Sa", "Su"];

/** The guest books from Lisbon; the host works in Berlin, one hour ahead. */
const GUEST_TIMES = ["09:00", "09:30", "10:00", "10:30", "13:00", "13:30"];
const PICKED_TIME = "10:00";

type HostRow = { hour: number; name?: string; picked?: boolean };
const HOST_DAY: HostRow[] = [
  { hour: 9, name: "Mia" },
  { hour: 10 },
  { hour: 11, picked: true },
  { hour: 12, name: "" },
  { hour: 13, name: "Noor" },
  { hour: 14 },
  { hour: 15 },
];
const GUEST = "Leo";

function usePrefersReducedMotion(): boolean {
  const [reduced, setReduced] = useState(false);
  useEffect(() => {
    const query = window.matchMedia("(prefers-reduced-motion: reduce)");
    setReduced(query.matches);
    const update = () => setReduced(query.matches);
    query.addEventListener("change", update);
    return () => query.removeEventListener("change", update);
  }, []);
  return reduced;
}

/** Steps through the beats in a loop while `running`. */
function useBeat(running: boolean, still: boolean): Step {
  const [step, setStep] = useState<Step>(STEP.IDLE);
  useEffect(() => {
    if (still || !running) return;
    const timer = window.setTimeout(
      () =>
        setStep((current) => (current === LAST_STEP ? STEP.IDLE : current + 1) as Step),
      BEAT_MS[step],
    );
    return () => window.clearTimeout(timer);
  }, [step, running, still]);
  return still ? LAST_STEP : step;
}

type Box = { x: number; y: number; width: number; height: number };

function boxIn(stage: HTMLElement, target: Element | null): Box | null {
  if (!target) return null;
  const outer = stage.getBoundingClientRect();
  const inner = target.getBoundingClientRect();
  return {
    x: inner.left - outer.left,
    y: inner.top - outer.top,
    width: inner.width,
    height: inner.height,
  };
}

type Targets = Record<
  "day" | "slot" | "confirm" | "host",
  RefObject<HTMLSpanElement | null>
>;
type Boxes = Partial<Record<keyof Targets, Box>>;

/** Where each target sits in the stage, kept current as the layout reflows. */
function useBoxes(stage: RefObject<HTMLElement | null>, targets: Targets): Boxes {
  const [boxes, setBoxes] = useState<Boxes>({});
  const measure = useCallback(() => {
    const element = stage.current;
    if (!element) return;
    const next: Boxes = {};
    for (const key of Object.keys(targets) as (keyof Targets)[]) {
      const box = boxIn(element, targets[key].current);
      if (box) next[key] = box;
    }
    setBoxes(next);
  }, [stage, targets]);

  useLayoutEffect(() => {
    measure();
    const element = stage.current;
    if (!element) return;
    const observer = new ResizeObserver(measure);
    observer.observe(element);
    return () => observer.disconnect();
  }, [measure, stage]);
  return boxes;
}

function Cursor({ step, boxes }: { step: Step; boxes: Boxes }) {
  const target =
    step === STEP.DAY
      ? boxes.day
      : step === STEP.TIME
        ? boxes.slot
        : step >= STEP.CONFIRM
          ? boxes.confirm
          : undefined;
  // Idle, it rests just below the calendar, where a hand would come from.
  const point = target
    ? { x: target.x + target.width * 0.6, y: target.y + target.height * 0.55 }
    : boxes.day
      ? { x: boxes.day.x + 70, y: boxes.day.y + 120 }
      : null;
  if (!point) return null;
  return (
    <span
      className="pointer-events-none absolute top-0 left-0 z-20 transition-[translate,opacity] ease-[cubic-bezier(0.45,0,0.2,1)]"
      style={{
        translate: `${point.x}px ${point.y}px`,
        opacity: step >= STEP.FLY ? 0 : 1,
        transitionDuration: `${GLIDE_MS}ms, 300ms`,
      }}
    >
      {/* Remounted every beat, so the press replays once the glide ends. */}
      <svg
        key={step}
        viewBox="0 0 20 20"
        className="size-5 drop-shadow-sm motion-safe:animate-cursor-click"
        style={{ animationDelay: `${GLIDE_MS}ms` }}
      >
        <path
          d="M3 2 L3 16 L7 12.5 L9.6 18 L12 17 L9.5 11.5 L15 11.5 Z"
          className="fill-foreground stroke-background"
          strokeWidth="1.25"
          strokeLinejoin="round"
        />
      </svg>
    </span>
  );
}

/** The booked block on its way from the guest's pick into the host's day. */
function Flyer({ step, boxes }: { step: Step; boxes: Boxes }) {
  const from = boxes.slot;
  const to = boxes.host;
  if (!from || !to) return null;
  const box = step >= STEP.FLY ? to : from;
  const flying = step === STEP.FLY;
  return (
    <span
      className="pointer-events-none absolute top-0 left-0 z-10 flex items-center rounded-md bg-dusk px-3 text-sm font-medium text-dusk-foreground shadow-lg"
      style={{
        translate: `${box.x}px ${box.y}px`,
        width: box.width,
        height: box.height,
        opacity: flying ? 1 : 0,
        transition: flying
          ? "translate 750ms cubic-bezier(0.5,0,0.2,1), width 750ms cubic-bezier(0.5,0,0.2,1), height 750ms cubic-bezier(0.5,0,0.2,1)"
          : "none",
      }}
    >
      {GUEST}
    </span>
  );
}

function GuestPane({ step, targets }: { step: Step; targets: Targets }) {
  const days = monthWeeks(YEAR, MONTH, { rows: "fit" }).flat();
  const pickedWeek = Math.floor(
    days.findIndex((cell) => cell.inMonth && cell.day === PICKED_DAY) / 7,
  );
  const dayPicked = step >= STEP.DAY;
  const timePicked = step >= STEP.TIME;
  const booked = step >= STEP.FLY;
  return (
    <div className="flex flex-col rounded-2xl border bg-card shadow-card">
      <div className="flex items-center gap-2 border-b px-4 py-2.5 text-sm text-muted-foreground">
        <Link2 className="size-4 shrink-0" />
        <span className="truncate">
          {APP_NAME.toLowerCase()}/book/<span className="text-foreground">ana</span>
        </span>
      </div>
      <div className="grid gap-5 p-5 sm:grid-cols-[minmax(0,17rem)_minmax(0,1fr)] sm:gap-10">
        <div>
          <div className="flex items-center gap-3">
            <span className="flex size-9 items-center justify-center rounded-full bg-dusk text-sm font-semibold text-dusk-foreground">
              A
            </span>
            <div>
              <p className="text-sm text-muted-foreground">Ana Kim</p>
              <p className="font-display text-lg leading-tight font-semibold">
                Intro call, 30 min
              </p>
            </div>
          </div>
          <p className="mt-5 font-display font-semibold">{monthLabel(YEAR, MONTH)}</p>
          <div className="mt-2 grid grid-cols-7 gap-1 text-center text-xs text-muted-foreground">
            {WEEKDAYS.map((weekday) => (
              <span key={weekday}>{weekday}</span>
            ))}
          </div>
          <div className="mt-1 grid grid-cols-7 gap-1">
            {days.map((cell, index) => {
              if (!cell.inMonth)
                return (
                  <span
                    key={`blank-${index}`}
                    className={
                      Math.floor(index / 7) !== pickedWeek ? "max-sm:hidden" : undefined
                    }
                  />
                );
              const weekday = index % 7;
              const open = weekday < 5 && cell.day >= 2;
              const picked = dayPicked && cell.day === PICKED_DAY;
              // Phones show only the picked week, as a strip.
              const otherWeek = Math.floor(index / 7) !== pickedWeek;
              return (
                <span
                  key={cell.day}
                  ref={cell.day === PICKED_DAY ? targets.day : undefined}
                  className={cn(
                    "flex aspect-square items-center justify-center rounded-md text-xs tabular-nums transition-colors duration-200",
                    open ? "bg-muted font-semibold" : "text-muted-foreground/50",
                    otherWeek && "max-sm:hidden",
                    picked && "bg-dusk text-dusk-foreground",
                  )}
                >
                  {cell.day}
                </span>
              );
            })}
          </div>
        </div>

        <div className="flex flex-col">
          <p className="flex items-center gap-1.5 text-sm text-muted-foreground">
            <Globe className="size-3.5" />
            Lisbon time
          </p>
          <div className="mt-3 grid flex-1 grid-cols-3 content-start gap-1.5 sm:grid-cols-1">
            {GUEST_TIMES.map((time, index) => {
              const picked = timePicked && time === PICKED_TIME;
              const phase = PHASE_STYLES[dayPhase(Number(time.slice(0, 2)) + 1)];
              return (
                <span
                  key={`${step >= STEP.DAY}-${time}`}
                  ref={time === PICKED_TIME ? targets.slot : undefined}
                  style={{ animationDelay: `${index * 60}ms` } as CSSProperties}
                  className={cn(
                    "relative flex h-8 items-center justify-center overflow-hidden rounded-md border text-clock text-sm font-medium",
                    dayPicked ? "motion-safe:animate-rise" : "invisible",
                  )}
                >
                  <span
                    className={cn(
                      "absolute inset-y-0 left-0 transition-[width] duration-300",
                      phase.rail,
                      picked ? "w-full" : "w-[3px]",
                    )}
                  />
                  <span className={cn("relative", picked && phase.picked)}>{time}</span>
                </span>
              );
            })}
          </div>
          <span
            ref={targets.confirm}
            className={cn(
              "mt-3 flex h-9 items-center justify-center gap-1.5 rounded-md text-sm font-medium transition-[background-color,color,scale] duration-200",
              booked
                ? "bg-success/15 text-success"
                : timePicked
                  ? "bg-primary text-primary-foreground"
                  : "bg-muted text-muted-foreground",
              step === STEP.CONFIRM && "motion-safe:scale-[0.97]",
            )}
          >
            {booked ? (
              <>
                <Check className="size-4" />
                Booked
              </>
            ) : (
              "Confirm"
            )}
          </span>
        </div>
      </div>
    </div>
  );
}

function HostPane({ step, targets }: { step: Step; targets: Targets }) {
  const landed = step >= STEP.BOOKED;
  return (
    <div className="flex flex-col rounded-2xl border bg-card shadow-card">
      <div className="flex items-baseline justify-between gap-3 border-b px-4 py-2.5">
        <p className="text-sm font-semibold">Your Tuesday</p>
        <p className="text-sm text-muted-foreground">Berlin time</p>
      </div>
      <ol className="relative grid gap-1.5 p-4">
        {HOST_DAY.map((row) => {
          const label = `${String(row.hour).padStart(2, "0")}:00`;
          const name = row.picked ? (landed ? GUEST : undefined) : row.name;
          return (
            <li key={row.hour} className="flex items-center gap-3">
              <span className="w-11 text-clock text-sm text-muted-foreground">
                {label}
              </span>
              {row.name === "" ? (
                <span className="h-9 flex-1 border-t border-dashed self-center" />
              ) : (
                <span
                  ref={row.picked ? targets.host : undefined}
                  className={cn(
                    "relative flex h-9 flex-1 items-center rounded-md border px-3 text-sm font-medium",
                    OPEN_STYLES[dayPhase(row.hour)],
                  )}
                >
                  {name ? (
                    // The flyer lands exactly here, so the block appears without a drop.
                    <span
                      className={cn(
                        "absolute inset-0 flex items-center justify-between rounded-md px-3",
                        // The new call in dusk; the ones already there stay quiet ink.
                        row.picked
                          ? "bg-dusk text-dusk-foreground"
                          : "bg-secondary text-secondary-foreground ring-1 ring-border",
                      )}
                    >
                      {name}
                      {row.picked ? <GoogleMeetIcon className="size-4" /> : null}
                    </span>
                  ) : (
                    <span className="text-foreground/60">open</span>
                  )}
                </span>
              )}
            </li>
          );
        })}
        {/* "Now", in the signal red of the logo's clock hand. */}
        <span
          aria-hidden
          className="pointer-events-none absolute top-[calc(1rem+2*2.625rem-0.25rem)] right-4 left-[4.5rem] flex items-center"
        >
          <span className="size-2 -translate-x-1 rounded-full bg-signal" />
          <span className="h-0.5 flex-1 bg-signal" />
        </span>
      </ol>
      <p
        className={cn(
          "mx-4 mb-4 flex items-center gap-2 rounded-md bg-success/10 px-3 py-2 text-sm font-medium text-success transition-opacity duration-300",
          landed ? "opacity-100" : "opacity-0",
        )}
      >
        <Check className="size-4 shrink-0" />
        On your calendar, with a Meet link
      </p>
    </div>
  );
}

/**
 * The landing page's hero: a guest picks a time on Ana's link, and the call
 * lands in Ana's day. Scripted and illustrative, so assistive tech skips it.
 * It pauses off screen and under the pointer; with reduced motion it is the
 * last frame, held still.
 */
export function BookingDemo() {
  const stage = useRef<HTMLDivElement>(null);
  const day = useRef<HTMLSpanElement>(null);
  const slot = useRef<HTMLSpanElement>(null);
  const confirm = useRef<HTMLSpanElement>(null);
  const host = useRef<HTMLSpanElement>(null);
  const [targets] = useState<Targets>(() => ({ day, slot, confirm, host }));
  const [hovered, setHovered] = useState(false);
  const onScreen = useOnScreen(stage);
  const still = usePrefersReducedMotion();
  const step = useBeat(onScreen && !hovered, still);
  const boxes = useBoxes(stage, targets);

  return (
    <div
      ref={stage}
      aria-hidden="true"
      onMouseEnter={() => setHovered(true)}
      onMouseLeave={() => setHovered(false)}
      // Stacked, the panes keep a readable width instead of stretching to the edge.
      className="relative grid max-w-2xl gap-4 select-none lg:max-w-none lg:grid-cols-[minmax(0,7fr)_minmax(0,5fr)] lg:gap-6"
    >
      <GuestPane step={step} targets={targets} />
      <HostPane step={step} targets={targets} />
      {still ? null : (
        <>
          <Flyer step={step} boxes={boxes} />
          <Cursor step={step} boxes={boxes} />
        </>
      )}
    </div>
  );
}
