import { useMutation, useQueryClient } from "@tanstack/react-query";
import { Loader2, Trash2 } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { toast } from "sonner";

import type { AvailabilityRule } from "../../bindings/AvailabilityRule";
import { Button } from "../../components/ui/button";
import { Checkbox } from "../../components/ui/checkbox";
import { Label } from "../../components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "../../components/ui/select";
import { errorMessage } from "../../lib/error-message";
import { clockMinutes, clockTime, weekdayName } from "../../lib/format";
import { QUERY_KEYS } from "../../lib/query-keys";
import { createAvailabilityRule, updateAvailabilityRule } from "../dashboard/host-api";
import { Field } from "../shared/field";
import { TimePicker } from "../shared/time-picker";

export const WEEKDAYS = [0, 1, 2, 3, 4, 5, 6];
export const SLOT_MINUTES = [15, 20, 30, 45, 60];
const DEFAULT_HOURS = { start: "09:00", end: "17:00", slotMinutes: 30 };
/** The latest time the picker offers. */
export const LAST_TIME = "23:45";
const LAST_HOUR = 23;

/** One hour after `time`, kept inside the day. */
export function hourAfter(time: string): string {
  const minutes = clockMinutes(time);
  return minutes >= LAST_HOUR * 60 ? LAST_TIME : clockTime(minutes + 60);
}

/** Adds hours to a day, or changes the hours in `rule`. */
export function HoursForm({
  weekday,
  rule,
  initial = DEFAULT_HOURS,
  onRemove,
  onDone,
}: {
  weekday: number;
  rule?: AvailabilityRule;
  /** Where new hours start out: a tapped hour, or an ordinary working day. */
  initial?: { start: string; end: string; slotMinutes: number };
  /** Shown while changing `rule`, to take it away. */
  onRemove?: () => void;
  onDone: () => void;
}) {
  const queryClient = useQueryClient();
  const [start, setStart] = useState(
    rule ? clockTime(clockMinutes(rule.start_time)) : initial.start,
  );
  const [end, setEnd] = useState(
    rule ? clockTime(clockMinutes(rule.end_time)) : initial.end,
  );
  const [slotMinutes, setSlotMinutes] = useState(
    rule?.slot_minutes ?? initial.slotMinutes,
  );
  const [otherDays, setOtherDays] = useState<number[]>([]);
  const endsAfterStart = start < end;
  const id = `hours-${weekday}-${rule?.id ?? "new"}`;
  const hours = (day: number) => ({
    weekday: day,
    start_time: `${start}:00`,
    end_time: `${end}:00`,
    slot_minutes: slotMinutes,
  });

  // Days already saved with these exact hours. A retry after one day failed
  // must not add the others a second time.
  const savedDays = useRef(new Set<number>());
  // biome-ignore lint/correctness/useExhaustiveDependencies: different hours are a different save
  useEffect(() => savedDays.current.clear(), [start, end, slotMinutes]);
  const formRef = useRef<HTMLFormElement>(null);
  // The form replaces the control that opened it, so it takes the focus.
  useEffect(() => formRef.current?.querySelector("button")?.focus(), []);

  const saveHours = useMutation({
    mutationFn: () =>
      Promise.all(
        [weekday, ...otherDays]
          .filter((day) => !savedDays.current.has(day))
          .map(async (day) => {
            if (rule && day === weekday)
              await updateAvailabilityRule(rule.id, hours(day));
            else await createAvailabilityRule(hours(day));
            savedDays.current.add(day);
          }),
      ),
    onSuccess: () => {
      toast.success(rule ? "Hours saved" : "Hours added");
      onDone();
    },
    onError: (error) =>
      toast.error(
        otherDays.length > 0
          ? `Some days were not saved. ${errorMessage(error)}`
          : `Hours were not ${rule ? "saved" : "added"}. ${errorMessage(error)}`,
      ),
    // Some days may have been saved even when one failed.
    onSettled: () => queryClient.invalidateQueries({ queryKey: QUERY_KEYS.availability }),
  });
  const submitLabel = rule
    ? saveHours.isPending
      ? "Saving"
      : "Save hours"
    : saveHours.isPending
      ? "Adding"
      : "Add hours";

  return (
    <form
      ref={formRef}
      className="grid gap-3 rounded-lg border bg-muted p-3"
      onSubmit={(event) => {
        event.preventDefault();
        if (endsAfterStart) saveHours.mutate();
      }}
    >
      <div className="flex flex-wrap items-end gap-3">
        <Field id={`${id}-start`} label="From">
          <TimePicker
            id={`${id}-start`}
            value={start}
            before={LAST_TIME}
            onChange={(time) => {
              setStart(time);
              if (time >= end) setEnd(hourAfter(time));
            }}
          />
        </Field>
        <Field id={`${id}-end`} label="To">
          <TimePicker
            id={`${id}-end`}
            value={end}
            after={start}
            invalid={!endsAfterStart}
            onChange={setEnd}
          />
        </Field>
        <Field id={`${id}-length`} label="Call length">
          <Select
            value={String(slotMinutes)}
            onValueChange={(value) => setSlotMinutes(Number(value))}
          >
            <SelectTrigger id={`${id}-length`} className="w-28 bg-card">
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
        </Field>
      </div>
      <fieldset className="flex flex-wrap items-center gap-x-4 gap-y-2">
        <legend className="float-left mr-4 text-sm font-medium">Also add to</legend>
        {WEEKDAYS.filter((day) => day !== weekday).map((day) => (
          <Label key={day} className="font-normal">
            <Checkbox
              className="bg-card"
              checked={otherDays.includes(day)}
              onCheckedChange={(checked) =>
                setOtherDays(
                  checked === true
                    ? [...otherDays, day]
                    : otherDays.filter((other) => other !== day),
                )
              }
            />
            <span aria-hidden="true">{weekdayName(day).slice(0, 3)}</span>
            <span className="sr-only">{weekdayName(day)}</span>
          </Label>
        ))}
      </fieldset>
      {endsAfterStart ? null : (
        <p role="alert" className="text-sm text-destructive">
          The end time must be after the start.
        </p>
      )}
      <div className="flex flex-wrap justify-end gap-2">
        {rule && onRemove ? (
          <Button
            type="button"
            variant="ghost"
            className="mr-auto text-destructive hover:text-destructive"
            onClick={onRemove}
          >
            <Trash2 />
            Remove hours
          </Button>
        ) : null}
        <Button type="button" variant="ghost" onClick={onDone}>
          Cancel
        </Button>
        <Button type="submit" disabled={saveHours.isPending || !endsAfterStart}>
          {saveHours.isPending ? <Loader2 className="animate-spin" /> : null}
          {submitLabel}
        </Button>
      </div>
    </form>
  );
}
