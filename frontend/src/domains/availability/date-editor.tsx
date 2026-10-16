import { useMutation, useQueryClient } from "@tanstack/react-query";
import { Loader2, Plus, X } from "lucide-react";
import { useState } from "react";
import { toast } from "sonner";

import type { AvailabilityRule } from "../../bindings/AvailabilityRule";
import type { DateOverride } from "../../bindings/DateOverride";
import { Button } from "../../components/ui/button";
import { Label } from "../../components/ui/label";
import { RadioGroup, RadioGroupItem } from "../../components/ui/radio-group";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "../../components/ui/select";
import { errorMessage } from "../../lib/error-message";
import {
  clockMinutes,
  clockTime,
  formatClock,
  formatLongDay,
  prefersHour12,
} from "../../lib/format";
import { QUERY_KEYS } from "../../lib/query-keys";
import { removeDateOverride, saveDateOverride } from "../dashboard/host-api";
import { TimePicker } from "../shared/time-picker";
import { LAST_TIME, SLOT_MINUTES, hourAfter } from "./hours-form";
import { DEFAULT_SLOT_MINUTES, hoursOn, ruleSpan, type Span } from "./hours-math";

const MODE = { weekly: "weekly", off: "off", own: "own" } as const;
type Mode = (typeof MODE)[keyof typeof MODE];
type Row = { key: number; start: string; end: string; slotMinutes: number };

/** "Saturday, October 24" for an ISO date, read as a calendar day. */
export function dateLabel(date: string): string {
  return formatLongDay(`${date}T12:00:00Z`, "UTC", false);
}

const rowOf = (span: Span, key: number): Row => ({
  key,
  start: clockTime(span.from),
  end: clockTime(span.to),
  slotMinutes: span.slotMinutes,
});

/** What the rows break: an end before its start, or two rows over the same time. */
function rowsProblem(rows: Row[]): string | null {
  if (rows.some((row) => row.start >= row.end))
    return "Each end time must be after its start.";
  const sorted = [...rows].sort((a, b) => a.start.localeCompare(b.start));
  if (
    sorted.some((row, index) => index > 0 && row.start < (sorted[index - 1]?.end ?? ""))
  )
    return "The hours on one date must not overlap.";
  return null;
}

/**
 * Sets one date apart from the weekly hours: off all day, or open at other
 * times. Choosing the weekly hours again removes what was set.
 */
export function DateEditor({
  date,
  rules,
  overrides,
  onDone,
}: {
  date: string;
  rules: AvailabilityRule[];
  overrides: DateOverride[];
  onDone: () => void;
}) {
  const queryClient = useQueryClient();
  const hour12 = prefersHour12();
  const own = overrides.find((day) => day.date === date);
  const weekly = hoursOn(date, rules, []).spans;
  const [mode, setMode] = useState<Mode>(
    own ? (own.ranges.length === 0 ? MODE.off : MODE.own) : MODE.weekly,
  );
  const startingRows = own?.ranges.length ? own.ranges.map(ruleSpan) : weekly;
  const [rows, setRows] = useState<Row[]>(
    startingRows.length > 0
      ? startingRows.map(rowOf)
      : [{ key: 0, start: "09:00", end: "17:00", slotMinutes: DEFAULT_SLOT_MINUTES }],
  );
  const problem = mode === MODE.own ? rowsProblem(rows) : null;
  const label = dateLabel(date);

  const save = useMutation({
    mutationFn: async () => {
      if (mode === MODE.weekly) {
        if (own) await removeDateOverride(date);
        return;
      }
      await saveDateOverride(date, {
        ranges:
          mode === MODE.off
            ? []
            : rows.map((row) => ({
                start_time: `${row.start}:00`,
                end_time: `${row.end}:00`,
                slot_minutes: row.slotMinutes,
              })),
      });
    },
    onSuccess: () => {
      toast.success(
        mode === MODE.weekly
          ? `${label} is back on the weekly hours`
          : mode === MODE.off
            ? `${label} is off`
            : `${label} saved`,
      );
      onDone();
    },
    onError: (error) => toast.error(`${label} was not saved. ${errorMessage(error)}`),
    onSettled: () =>
      queryClient.invalidateQueries({ queryKey: QUERY_KEYS.availabilityOverrides }),
  });

  const weeklySummary =
    weekly.length === 0
      ? "Off"
      : weekly
          .map(
            (span) =>
              `${formatClock(clockTime(span.from), hour12)} to ${formatClock(clockTime(span.to), hour12)}`,
          )
          .join(", ");
  const setRow = (key: number, change: Partial<Row>) =>
    setRows(rows.map((row) => (row.key === key ? { ...row, ...change } : row)));
  const id = `date-${date}`;

  return (
    <form
      className="grid gap-4"
      onSubmit={(event) => {
        event.preventDefault();
        if (!problem) save.mutate();
      }}
    >
      <h3 className="font-display text-base font-semibold tracking-tight">{label}</h3>
      <RadioGroup
        value={mode}
        onValueChange={(value) => setMode(value as Mode)}
        aria-label={`Hours on ${label}`}
      >
        <Label className="items-start font-normal">
          <RadioGroupItem value={MODE.weekly} className="mt-0.5" />
          <span className="grid gap-0.5">
            <span className="font-medium">Weekly hours</span>
            <span className="text-clock text-muted-foreground">{weeklySummary}</span>
          </span>
        </Label>
        <Label className="font-normal">
          <RadioGroupItem value={MODE.off} />
          <span className="font-medium">Off all day</span>
        </Label>
        <Label className="font-normal">
          <RadioGroupItem value={MODE.own} />
          <span className="font-medium">Other hours</span>
        </Label>
      </RadioGroup>

      {mode === MODE.own ? (
        <fieldset className="grid gap-2 rounded-lg border bg-muted p-3">
          <legend className="sr-only">Hours on {label}</legend>
          {rows.map((row, index) => (
            <div key={row.key} className="flex flex-wrap items-center gap-2">
              <TimePicker
                id={`${id}-${row.key}-start`}
                value={row.start}
                before={LAST_TIME}
                onChange={(start) =>
                  setRow(
                    row.key,
                    start >= row.end ? { start, end: hourAfter(start) } : { start },
                  )
                }
              />
              <span className="text-sm text-muted-foreground">to</span>
              <TimePicker
                id={`${id}-${row.key}-end`}
                value={row.end}
                after={row.start}
                invalid={row.start >= row.end}
                onChange={(end) => setRow(row.key, { end })}
              />
              <Select
                value={String(row.slotMinutes)}
                onValueChange={(value) => setRow(row.key, { slotMinutes: Number(value) })}
              >
                <SelectTrigger
                  className="w-24 bg-card"
                  aria-label={`Call length, range ${index + 1}`}
                >
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {SLOT_MINUTES.map((minutes) => (
                    <SelectItem key={minutes} value={String(minutes)}>
                      {minutes} min
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
              {rows.length > 1 ? (
                <Button
                  type="button"
                  variant="ghost"
                  size="icon-sm"
                  aria-label={`Remove range ${index + 1}`}
                  onClick={() => setRows(rows.filter((other) => other.key !== row.key))}
                >
                  <X />
                </Button>
              ) : null}
            </div>
          ))}
          <Button
            type="button"
            variant="ghost"
            size="sm"
            className="justify-self-start"
            onClick={() => {
              const last = rows.at(-1);
              const start =
                last && clockMinutes(last.end) < clockMinutes(LAST_TIME) - 60
                  ? hourAfter(last.end)
                  : "09:00";
              setRows([
                ...rows,
                {
                  key: Math.max(0, ...rows.map((row) => row.key)) + 1,
                  start,
                  end: hourAfter(start),
                  slotMinutes: last?.slotMinutes ?? DEFAULT_SLOT_MINUTES,
                },
              ]);
            }}
          >
            <Plus />
            Add hours
          </Button>
        </fieldset>
      ) : null}

      {problem ? (
        <p role="alert" className="text-sm text-destructive">
          {problem}
        </p>
      ) : null}
      <div className="flex justify-end gap-2">
        <Button type="button" variant="ghost" onClick={onDone}>
          Cancel
        </Button>
        <Button type="submit" disabled={save.isPending || problem !== null}>
          {save.isPending ? <Loader2 className="animate-spin" /> : null}
          {save.isPending ? "Saving" : "Save date"}
        </Button>
      </div>
    </form>
  );
}
