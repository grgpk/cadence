import { cn } from "cn";
import { Copy, Ellipsis, Plus, Trash2 } from "lucide-react";
import {
  useEffect,
  useRef,
  useState,
  type KeyboardEvent,
  type PointerEvent,
} from "react";

import type { AvailabilityRule } from "../../bindings/AvailabilityRule";
import { Button } from "../../components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "../../components/ui/dropdown-menu";
import {
  clockTime,
  formatClock,
  lengthLabel,
  prefersHour12,
  weekdayName,
} from "../../lib/format";
import { placeOnWeek, WeekRuler } from "../dashboard/week-grid";
import type { WeekHours } from "../dashboard/week-hours";
import { OPEN_STYLES } from "../landing/day-tape";
import { phaseSpans } from "../../lib/zoned-time";
import { HoursForm, WEEKDAYS, hourAfter } from "./hours-form";
import {
  DEFAULT_SLOT_MINUTES,
  SNAP_MINUTES,
  callsIn,
  dragSpan,
  foldInto,
  minutesAt,
  ruleSpan,
  type Bounds,
  type DragKind,
  type Span,
} from "./hours-math";
import { useHours } from "./use-hours";

/** Day name, the day's track, its calls: the same columns in every row. */
const WEEK_ROW = "grid gap-x-4 gap-y-2 px-5 sm:grid-cols-[8.5rem_minmax(0,1fr)_5.5rem]";
const LAST_MINUTE = 23 * 60 + 45;
/** Arrow keys may take a range anywhere in the day; the ruler grows to follow. */
const WHOLE_DAY: Bounds = { min: 0, max: LAST_MINUTE };
/** A press that travels less than this is a tap, not a drag. */
const DRAG_PIXELS = 4;
/** Arrow-key changes are saved once the keys go quiet for this long. */
const NUDGE_SAVE_MS = 700;
const WORK_DAYS = [0, 1, 2, 3, 4];
/** The cursor over the track while each kind of drag is under way. */
const DRAG_CURSOR: Record<DragKind, string> = {
  draw: "cursor-col-resize",
  move: "cursor-grabbing",
  start: "cursor-ew-resize",
  end: "cursor-ew-resize",
};

type Drag = {
  kind: DragKind;
  rule?: AvailabilityRule;
  /** The span, time and pixel when the press began. Every move is measured from these. */
  base: Span;
  origin: number;
  originX: number;
  /** Where the span is now. */
  span: Span;
  moved: boolean;
};
/** The form under a day: new hours (from a tap, or the menu), or one range's hours. */
type Form = { add: { start: string; end: string } | null } | { ruleId: number };

const blockId = (id: number) => `hours-block-${id}`;

function rangeLabel(span: Span, hour12: boolean): string {
  return `${formatClock(clockTime(span.from), hour12)} to ${formatClock(clockTime(span.to), hour12)}`;
}

const callsLabel = (calls: number) => `${calls} ${calls === 1 ? "call" : "calls"}`;

function DayMenu({
  weekday,
  rules,
  onAdd,
}: {
  weekday: number;
  rules: AvailabilityRule[];
  onAdd: () => void;
}) {
  const { remove, copyDay } = useHours();
  const name = weekdayName(weekday);
  const copyTo = (days: number[]) =>
    copyDay.mutate({ source: weekday, days: days.filter((day) => day !== weekday) });
  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button
          type="button"
          variant="ghost"
          size="icon-sm"
          aria-label={`${name} options`}
        >
          <Ellipsis />
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="start">
        <DropdownMenuItem onSelect={onAdd}>
          <Plus />
          Add hours
        </DropdownMenuItem>
        {rules.length > 0 ? (
          <>
            <DropdownMenuSeparator />
            {WORK_DAYS.includes(weekday) ? (
              <DropdownMenuItem onSelect={() => copyTo(WORK_DAYS)}>
                <Copy />
                Copy to Mon to Fri
              </DropdownMenuItem>
            ) : null}
            <DropdownMenuItem onSelect={() => copyTo(WEEKDAYS)}>
              <Copy />
              Copy to every day
            </DropdownMenuItem>
            <DropdownMenuSeparator />
            <DropdownMenuItem variant="destructive" onSelect={() => remove.mutate(rules)}>
              <Trash2 />
              Clear {name}
            </DropdownMenuItem>
          </>
        ) : null}
      </DropdownMenuContent>
    </DropdownMenu>
  );
}

/** One range on a day's track: a button that can be dragged, stretched, or nudged. */
function Block({
  rule,
  span,
  week,
  look,
  label,
  onPress,
  onKeyDown,
  onBlur,
}: {
  rule?: AvailabilityRule;
  span: Span;
  week: WeekHours;
  /** Held by the pointer, about to be folded into the held range, or at rest. */
  look: "held" | "joining" | "rest";
  label: string;
  onPress?: (kind: DragKind, event: PointerEvent) => void;
  onKeyDown?: (event: KeyboardEvent) => void;
  onBlur?: () => void;
}) {
  const hour12 = prefersHour12();
  const saving = rule !== undefined && rule.id < 0;
  const grip = (kind: "start" | "end") => (
    <span
      aria-hidden="true"
      onPointerDown={(event) => onPress?.(kind, event)}
      className={cn(
        "absolute inset-y-0 z-10 flex w-3 cursor-ew-resize items-center justify-center pointer-coarse:w-6",
        kind === "start" ? "left-0" : "right-0",
      )}
    >
      <span
        className={cn(
          "h-5 w-1 rounded-full bg-foreground/0 transition-colors group-hover/block:bg-foreground/40 group-focus-visible/block:bg-foreground/40",
          look === "held" && "bg-dusk/80 group-hover/block:bg-dusk/80",
        )}
      />
    </span>
  );
  return (
    <button
      id={rule ? blockId(rule.id) : undefined}
      type="button"
      tabIndex={rule && !saving ? 0 : -1}
      aria-label={label}
      aria-keyshortcuts={
        rule
          ? "ArrowLeft ArrowRight Shift+ArrowLeft Shift+ArrowRight Alt+ArrowLeft Alt+ArrowRight Enter Delete"
          : undefined
      }
      style={placeOnWeek(week, span.from, span.to)}
      onPointerDown={(event) => onPress?.("move", event)}
      onKeyDown={onKeyDown}
      onBlur={onBlur}
      className={cn(
        "group/block absolute inset-y-1 flex cursor-grab items-center overflow-hidden rounded-md outline-none select-none",
        // The snap from one quarter hour to the next glides instead of jumping.
        "transition-[left,width,box-shadow] duration-100 ease-out motion-reduce:transition-none",
        "focus-visible:ring-[3px] focus-visible:ring-ring/50",
        look === "held" && "z-20 shadow-lg ring-2 ring-dusk",
        look === "joining" &&
          "outline-2 outline-offset-[-2px] outline-dusk outline-dashed",
        saving && "cursor-progress opacity-70",
        !rule && "pointer-events-none",
      )}
    >
      {phaseSpans(span.from, span.to).map((part) => (
        <span
          key={part.phase}
          style={{
            left: `${((part.from - span.from) / (span.to - span.from)) * 100}%`,
            width: `${((part.to - part.from) / (span.to - span.from)) * 100}%`,
          }}
          className={cn("absolute inset-y-0 border-y", OPEN_STYLES[part.phase])}
        />
      ))}
      <span className="relative truncate px-3 text-clock text-sm font-medium">
        {rangeLabel(span, hour12)}
      </span>
      {rule && !saving ? (
        <>
          {grip("start")}
          {grip("end")}
        </>
      ) : null}
    </button>
  );
}

/**
 * A label above the track at `minute`. It leans toward the middle, so it
 * stays over the track at both ends.
 */
function Readout({
  week,
  minute,
  strong,
  children,
}: {
  week: WeekHours;
  minute: number;
  strong: boolean;
  children: string;
}) {
  const share = Math.min(
    100,
    Math.max(0, ((minute - week.first * 60) / ((week.last - week.first) * 60)) * 100),
  );
  return (
    <span
      style={{ left: `${share}%`, transform: `translateX(-${share}%)` }}
      className={cn(
        "pointer-events-none absolute -top-7 z-30 rounded-md px-2 py-0.5 text-clock text-xs font-medium whitespace-nowrap shadow-sm",
        strong
          ? "bg-primary text-primary-foreground"
          : "border bg-popover text-popover-foreground",
      )}
    >
      {children}
    </span>
  );
}

function DayRow({
  weekday,
  rules,
  week,
}: {
  weekday: number;
  rules: AvailabilityRule[];
  week: WeekHours;
}) {
  const { saveSpan, remove } = useHours();
  const hour12 = prefersHour12();
  const name = weekdayName(weekday);
  const trackRef = useRef<HTMLDivElement>(null);
  const [drag, setDrag] = useState<Drag | null>(null);
  // The time under a mouse resting on an empty part of the track.
  const [hover, setHover] = useState<number | null>(null);
  // Arrow keys move a range here first; it is saved when the keys go quiet.
  const [nudge, setNudge] = useState<{ rule: AvailabilityRule; span: Span } | null>(null);
  const nudgeTimer = useRef<number | undefined>(undefined);
  const [form, setForm] = useState<Form | null>(null);
  const formRuleId = form && "ruleId" in form ? form.ruleId : null;
  const slotMinutes = rules[0]?.slot_minutes ?? DEFAULT_SLOT_MINUTES;
  // A pointer drag stays on the ruler that is drawn.
  const bounds: Bounds = {
    min: week.first * 60,
    max: Math.min(week.last * 60, LAST_MINUTE),
  };

  /** Saves `span` on this day, folding in the ranges it now touches. */
  const save = (span: Span, self?: AvailabilityRule) => {
    const folded = foldInto(rules, span.from, span.to, self);
    saveSpan.mutate({
      weekday,
      keep: folded.keep,
      from: folded.from,
      to: folded.to,
      slotMinutes: folded.keep?.slot_minutes ?? span.slotMinutes,
      drop: folded.drop,
    });
  };

  const commitNudge = () => {
    window.clearTimeout(nudgeTimer.current);
    if (nudge) save(nudge.span, nudge.rule);
    setNudge(null);
  };
  useEffect(() => () => window.clearTimeout(nudgeTimer.current), []);

  // Escape lets go of a drag and leaves the hours as they were.
  const dragging = drag !== null;
  useEffect(() => {
    if (!dragging) return;
    const cancel = (event: globalThis.KeyboardEvent) => {
      if (event.key === "Escape") setDrag(null);
    };
    window.addEventListener("keydown", cancel);
    return () => window.removeEventListener("keydown", cancel);
  }, [dragging]);

  const at = (clientX: number) => {
    const track = trackRef.current?.getBoundingClientRect();
    return track
      ? Math.min(LAST_MINUTE, minutesAt(clientX, track.left, track.width, week))
      : 0;
  };

  const press = (kind: DragKind, event: PointerEvent, rule?: AvailabilityRule) => {
    if (event.button !== 0 || !trackRef.current) return;
    event.stopPropagation();
    // A range still being saved has no id to change yet.
    if (rule && rule.id < 0) return;
    commitNudge();
    // A drag on a touch screen draws or moves; scrolling the page needs a vertical swipe.
    trackRef.current.setPointerCapture(event.pointerId);
    const origin = at(event.clientX);
    const base = rule
      ? ruleSpan(rule)
      : { from: origin, to: origin + SNAP_MINUTES, slotMinutes };
    setHover(null);
    setDrag({
      kind,
      rule,
      base,
      origin,
      originX: event.clientX,
      span: base,
      moved: false,
    });
  };

  const onPointerMove = (event: PointerEvent) => {
    if (!drag) {
      // Only a resting mouse over empty track shows where a drag would start.
      const empty =
        event.pointerType === "mouse" &&
        event.buttons === 0 &&
        !(event.target as Element).closest("button");
      setHover(empty ? at(event.clientX) : null);
      return;
    }
    const moved = drag.moved || Math.abs(event.clientX - drag.originX) > DRAG_PIXELS;
    if (!moved) return;
    const span = dragSpan(drag.kind, drag.base, drag.origin, at(event.clientX), bounds);
    if (drag.moved && span.from === drag.span.from && span.to === drag.span.to) return;
    setDrag({ ...drag, moved, span });
  };

  const onPointerUp = () => {
    if (!drag) return;
    setDrag(null);
    if (drag.moved) {
      const same = drag.span.from === drag.base.from && drag.span.to === drag.base.to;
      if (!(drag.rule && same)) save(drag.span, drag.rule);
      return;
    }
    // A tap: open the range, or start new hours at the hour under the finger.
    if (drag.rule) {
      setForm({ ruleId: drag.rule.id });
      return;
    }
    const start = clockTime(
      Math.min(Math.floor(drag.origin / 60) * 60, LAST_MINUTE - 60),
    );
    setForm({ add: { start, end: hourAfter(start) } });
  };

  const onBlockKey = (rule: AvailabilityRule) => (event: KeyboardEvent) => {
    if (event.key === "Enter" || event.key === " ") {
      event.preventDefault();
      commitNudge();
      setForm({ ruleId: rule.id });
      return;
    }
    if (event.key === "Delete" || event.key === "Backspace") {
      event.preventDefault();
      setNudge(null);
      remove.mutate([rule]);
      return;
    }
    if (event.key === "Escape" && nudge) {
      window.clearTimeout(nudgeTimer.current);
      setNudge(null);
      return;
    }
    const step = { ArrowLeft: -SNAP_MINUTES, ArrowRight: SNAP_MINUTES }[event.key];
    if (step === undefined) return;
    event.preventDefault();
    const span = nudge?.rule.id === rule.id ? nudge.span : ruleSpan(rule);
    const kind: DragKind = event.shiftKey ? "end" : event.altKey ? "start" : "move";
    const edge = kind === "end" ? span.to : span.from;
    const next = dragSpan(kind, span, edge, edge + step, WHOLE_DAY);
    setNudge({ rule, span: next });
    window.clearTimeout(nudgeTimer.current);
    nudgeTimer.current = window.setTimeout(() => {
      save(next, rule);
      setNudge(null);
    }, NUDGE_SAVE_MS);
  };

  // When the form closes, focus goes back to the range it changed, or to the menu.
  const lastForm = useRef<Form | null>(null);
  useEffect(() => {
    if (form) {
      lastForm.current = form;
      return;
    }
    const closed = lastForm.current;
    lastForm.current = null;
    if (closed && "ruleId" in closed)
      document.getElementById(blockId(closed.ruleId))?.focus();
  }, [form]);

  const held = drag?.moved ? drag : null;
  const live = held?.span ?? nudge?.span ?? null;
  const liveRule = held ? held.rule : nudge?.rule;
  // What letting go would save: the live span with the ranges it touches folded in.
  const folded = live ? foldInto(rules, live.from, live.to, liveRule) : null;
  const joining = new Set(
    folded
      ? [...folded.drop, ...(liveRule ? [] : folded.keep ? [folded.keep] : [])].map(
          (rule) => rule.id,
        )
      : [],
  );
  const result: Span | null = folded
    ? {
        from: folded.from,
        to: folded.to,
        slotMinutes: folded.keep?.slot_minutes ?? live?.slotMinutes ?? slotMinutes,
      }
    : null;

  const spanOf = (rule: AvailabilityRule) => {
    if (held?.rule?.id === rule.id) return held.span;
    if (nudge?.rule.id === rule.id) return nudge.span;
    return ruleSpan(rule);
  };
  const shown = rules.map((rule) => ({ rule, span: spanOf(rule) }));
  const drawing = held?.kind === "draw" ? held.span : null;
  const calls = result
    ? shown
        .filter(({ rule }) => rule.id !== liveRule?.id && !joining.has(rule.id))
        .reduce((total, { span }) => total + callsIn(span), callsIn(result))
    : shown.reduce((total, { span }) => total + callsIn(span), 0);
  const describe = (span: Span) =>
    `${name} ${rangeLabel(span, hour12)}, ${span.slotMinutes} min calls`;
  const formRule = rules.find((rule) => rule.id === formRuleId);
  const readout = result
    ? `${rangeLabel(result, hour12)}, ${lengthLabel(result.to - result.from)}, ${callsLabel(callsIn(result))}`
    : null;

  return (
    <div className={cn(WEEK_ROW, "py-3 sm:items-center")}>
      <div className="flex items-center gap-1">
        <h3 className="text-sm font-semibold">{name}</h3>
        <span className="ml-auto text-clock text-sm text-muted-foreground sm:hidden">
          {calls > 0 ? callsLabel(calls) : "Off"}
        </span>
        <span className="sm:order-first sm:-ml-2">
          <DayMenu weekday={weekday} rules={rules} onAdd={() => setForm({ add: null })} />
        </span>
      </div>
      <div className="grid gap-2.5">
        <div
          ref={trackRef}
          onPointerDown={(event) => press("draw", event)}
          onPointerMove={onPointerMove}
          onPointerUp={onPointerUp}
          onPointerCancel={() => setDrag(null)}
          onPointerLeave={() => setHover(null)}
          style={{
            backgroundImage:
              "linear-gradient(to right, color-mix(in oklab, var(--foreground) 7%, transparent) 1px, transparent 1px)",
            backgroundSize: `${100 / (week.last - week.first)}% 100%`,
          }}
          className={cn(
            "group/track relative h-12 touch-pan-y rounded-md bg-muted/70 select-none",
            drag ? DRAG_CURSOR[drag.kind] : "cursor-crosshair",
          )}
        >
          {/* Ranges never draw outside the track, wherever a drag or key takes them. */}
          <div
            className={cn(
              "absolute inset-0 overflow-hidden rounded-md",
              // While a drag is held the track owns the pointer and the cursor.
              drag && "pointer-events-none",
            )}
          >
            {rules.length === 0 && !drawing && hover === null ? (
              <span className="pointer-events-none absolute inset-0 flex items-center justify-center text-sm text-muted-foreground opacity-70 transition-opacity group-hover/track:opacity-100">
                <span className="pointer-coarse:hidden">Drag across to add hours</span>
                <span className="hidden pointer-coarse:inline">Tap to add hours</span>
              </span>
            ) : null}
            {held?.rule ? (
              // Where the range was when it was picked up.
              <span
                aria-hidden="true"
                style={placeOnWeek(week, held.base.from, held.base.to)}
                className="absolute inset-y-1 rounded-md border border-dashed border-foreground/30"
              />
            ) : null}
            {shown.map(({ rule, span }) => (
              <Block
                key={rule.id}
                rule={rule}
                span={span}
                week={week}
                look={
                  rule.id === liveRule?.id && live
                    ? "held"
                    : joining.has(rule.id)
                      ? "joining"
                      : "rest"
                }
                label={describe(span)}
                onPress={(kind, event) => press(kind, event, rule)}
                onKeyDown={onBlockKey(rule)}
                onBlur={commitNudge}
              />
            ))}
            {drawing ? (
              <Block span={drawing} week={week} look="held" label={describe(drawing)} />
            ) : null}
            {hover !== null && !drag ? (
              <span
                aria-hidden="true"
                style={{ left: placeOnWeek(week, hover, hover).left }}
                className="pointer-events-none absolute inset-y-0 w-px bg-foreground/50"
              />
            ) : null}
          </div>
          {readout && result ? (
            <Readout week={week} minute={(result.from + result.to) / 2} strong>
              {readout}
            </Readout>
          ) : hover !== null && !drag ? (
            <Readout week={week} minute={hover} strong={false}>
              {formatClock(clockTime(hover), hour12)}
            </Readout>
          ) : null}
        </div>
        <p aria-live="polite" className="sr-only">
          {nudge && readout ? readout : ""}
        </p>
        {form ? (
          <HoursForm
            // A different range is a different form, with its own starting values.
            key={"ruleId" in form ? form.ruleId : `add-${form.add?.start ?? ""}`}
            weekday={weekday}
            rule={formRule}
            initial={
              "add" in form && form.add
                ? { ...form.add, slotMinutes }
                : { start: "09:00", end: "17:00", slotMinutes }
            }
            onRemove={
              formRule
                ? () => {
                    remove.mutate([formRule]);
                    setForm(null);
                  }
                : undefined
            }
            onDone={() => setForm(null)}
          />
        ) : null}
      </div>
      <p className="hidden text-right text-clock text-sm sm:block">
        {calls > 0 ? (
          callsLabel(calls)
        ) : (
          <span className="text-muted-foreground">Off</span>
        )}
      </p>
    </div>
  );
}

/**
 * The weekly hours as one ruler with a track per day. Drag across a track to
 * open hours, drag a range to move it, drag its ends to stretch it.
 */
export function WeekCanvas({
  rules,
  week,
}: {
  rules: AvailabilityRule[];
  week: WeekHours;
}) {
  return (
    <div className="divide-y">
      <WeekRuler week={week} row={WEEK_ROW} />
      {WEEKDAYS.map((weekday) => (
        <DayRow
          key={weekday}
          weekday={weekday}
          rules={rules.filter((rule) => rule.weekday === weekday)}
          week={week}
        />
      ))}
    </div>
  );
}
