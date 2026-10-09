import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { createFileRoute, Link } from "@tanstack/react-router";
import { cn } from "cn";
import { Loader2, Plus, X } from "lucide-react";
import { useState } from "react";
import { toast } from "sonner";

import type { AvailabilityRule } from "../../bindings/AvailabilityRule";
import { Button } from "../../components/ui/button";
import { Card } from "../../components/ui/card";
import { Checkbox } from "../../components/ui/checkbox";
import { Label } from "../../components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "../../components/ui/select";
import {
  availabilityQuery,
  createAvailabilityRule,
  hostSettingsQuery,
  removeAvailabilityRule,
  updateAvailabilityRule,
} from "../../domains/dashboard/host-api";
import { weekHours } from "../../domains/dashboard/week-hours";
import { OPEN_STYLES } from "../../domains/landing/DayTape";
import { phaseSpans } from "../../domains/widget/zoned-time";
import { SETTINGS_SECTIONS } from "../../domains/dashboard/routes";
import { ErrorState } from "../../domains/shared/ErrorState";
import { Field } from "../../domains/shared/Field";
import { PageHeader } from "../../domains/shared/PageHeader";
import { TableSkeleton } from "../../domains/shared/TableSkeleton";
import { TimePicker } from "../../domains/shared/TimePicker";
import { APP_NAME } from "../../lib/config";
import { errorMessage } from "../../lib/error-message";
import {
  clockMinutes,
  clockTime,
  formatClock,
  prefersHour12,
  weekdayName,
  zoneName,
} from "../../lib/format";
import { QUERY_KEYS } from "../../lib/query-keys";
import { ROUTES } from "../../lib/routes";

export const Route = createFileRoute("/dashboard/availability")({
  head: () => ({ meta: [{ title: `Availability | ${APP_NAME}` }] }),
  component: AvailabilityPage,
});

const WEEKDAYS = [0, 1, 2, 3, 4, 5, 6];
const SLOT_MINUTES = [15, 20, 30, 45, 60];
const DEFAULT_HOURS = { start: "09:00", end: "17:00", slotMinutes: 30 };
/** The latest time the picker offers. */
const LAST_TIME = "23:45";
const LAST_HOUR = 23;

/** One hour after `time`, kept inside the day. */
function hourAfter(time: string): string {
  const minutes = clockMinutes(time);
  return minutes >= LAST_HOUR * 60 ? LAST_TIME : clockTime(minutes + 60);
}

/** Adds hours to a day, or changes the hours in `rule`. */
function HoursForm({
  weekday,
  rule,
  onDone,
}: {
  weekday: number;
  rule?: AvailabilityRule;
  onDone: () => void;
}) {
  const queryClient = useQueryClient();
  const [start, setStart] = useState(
    rule ? clockTime(clockMinutes(rule.start_time)) : DEFAULT_HOURS.start,
  );
  const [end, setEnd] = useState(
    rule ? clockTime(clockMinutes(rule.end_time)) : DEFAULT_HOURS.end,
  );
  const [slotMinutes, setSlotMinutes] = useState(
    rule?.slot_minutes ?? DEFAULT_HOURS.slotMinutes,
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

  const saveHours = useMutation({
    mutationFn: () =>
      Promise.all([
        rule
          ? updateAvailabilityRule(rule.id, hours(weekday))
          : createAvailabilityRule(hours(weekday)),
        ...otherDays.map((day) => createAvailabilityRule(hours(day))),
      ]),
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
      <div className="flex justify-end gap-2">
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

const HOUR = 60;
/** Day name, the week's ruler, the add button: the same columns in every row. */
const ROW = "grid gap-3 px-5 sm:grid-cols-[7rem_minmax(0,1fr)_7rem]";
/** Past this many hours the ruler labels every other one. */
const DENSE_HOURS = 12;

type WeekHours = { first: number; last: number };

/** Where a stretch of the day sits on the week's ruler. */
function place(week: WeekHours, from: number, to: number) {
  const span = (week.last - week.first) * HOUR;
  return {
    left: `${((from - week.first * HOUR) / span) * 100}%`,
    width: `${((to - from) / span) * 100}%`,
  };
}

/** The hours every day of the week is drawn against. */
function WeekRuler({ week }: { week: WeekHours }) {
  const hours = Array.from(
    { length: week.last - week.first },
    (_, index) => week.first + index,
  );
  const step = hours.length > DENSE_HOURS ? 2 : 1;
  return (
    <div aria-hidden="true" className={cn(ROW, "border-b bg-muted/40 pt-3 pb-0")}>
      <span className="hidden sm:block" />
      <div
        className="grid"
        style={{ gridTemplateColumns: `repeat(${hours.length}, minmax(0, 1fr))` }}
      >
        {hours.map((hour, index) => (
          <div key={hour}>
            <p className="h-5 text-clock text-sm font-medium text-muted-foreground">
              {index % step === 0 ? (
                // A phone has room for every other label.
                <span
                  className={index % (step * 2) === 0 ? undefined : "hidden sm:inline"}
                >
                  {String(hour).padStart(2, "0")}:00
                </span>
              ) : null}
            </p>
            <div className="relative mt-1 h-2.5 border-t border-foreground/25">
              <span className="absolute top-0 left-0 h-2.5 w-px bg-foreground/40" />
              <span className="absolute top-0 left-1/2 h-1.5 w-px bg-foreground/25" />
            </div>
          </div>
        ))}
      </div>
    </div>
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
  const queryClient = useQueryClient();
  // The form in this row: adding hours, or changing the rule with this id.
  const [form, setForm] = useState<"add" | number | null>(null);
  const name = weekdayName(weekday);

  const refresh = () =>
    queryClient.invalidateQueries({ queryKey: QUERY_KEYS.availability });

  const restoreHours = useMutation({
    mutationFn: createAvailabilityRule,
    onSuccess: async () => {
      await refresh();
      toast.success("Hours put back");
    },
    onError: (error) => toast.error(`Hours were not put back. ${errorMessage(error)}`),
  });
  const removeHours = useMutation({
    mutationFn: removeAvailabilityRule,
    onSuccess: async (_, rule) => {
      await refresh();
      toast.success("Hours removed", {
        action: {
          label: "Undo",
          onClick: () =>
            restoreHours.mutate({
              weekday: rule.weekday,
              start_time: rule.start_time,
              end_time: rule.end_time,
              slot_minutes: rule.slot_minutes,
            }),
        },
      });
    },
    onError: (error) => toast.error(`Hours were not removed. ${errorMessage(error)}`),
  });
  const hour12 = prefersHour12();
  const range = (rule: AvailabilityRule) =>
    `${formatClock(rule.start_time, hour12)} to ${formatClock(rule.end_time, hour12)}`;
  const editing = rules.find((rule) => rule.id === form);

  return (
    <div className={cn(ROW, "py-4 sm:items-start")}>
      <h2 className="pt-2 text-sm font-semibold">{name}</h2>
      <div className="grid gap-2.5">
        {/* The same hours as the list under it, so the picture is left to sighted pointer users. */}
        <div aria-hidden="true" className="relative h-9 rounded-md bg-muted/70">
          {rules.map((rule) => {
            const from = clockMinutes(rule.start_time);
            const to = clockMinutes(rule.end_time);
            return (
              <button
                key={rule.id}
                type="button"
                tabIndex={-1}
                style={place(week, from, to)}
                onClick={() => setForm(rule.id)}
                className="absolute inset-y-0 flex items-center overflow-hidden rounded-md"
              >
                {phaseSpans(from, to).map((span) => (
                  <span
                    key={span.phase}
                    style={{
                      left: `${((span.from - from) / (to - from)) * 100}%`,
                      width: `${((span.to - span.from) / (to - from)) * 100}%`,
                    }}
                    className={cn("absolute inset-y-0 border-y", OPEN_STYLES[span.phase])}
                  />
                ))}
                <span className="relative truncate px-2 text-clock text-sm font-medium">
                  {range(rule)}
                </span>
              </button>
            );
          })}
        </div>
        {rules.length === 0 && form === null ? (
          <p className="text-sm text-muted-foreground">Unavailable</p>
        ) : null}
        {rules.map((rule) =>
          rule.id === form ? null : (
            <div key={rule.id} className="flex items-center gap-3 text-sm">
              <button
                type="button"
                aria-label={`Edit ${name} ${range(rule)}`}
                onClick={() => setForm(rule.id)}
                className="rounded-md border bg-card px-2.5 py-1 text-clock text-[0.9375rem] font-medium transition-colors outline-none hover:border-foreground focus-visible:border-ring focus-visible:ring-[3px] focus-visible:ring-ring/50"
              >
                {range(rule)}
              </button>
              <span className="text-muted-foreground">{rule.slot_minutes} min calls</span>
              <Button
                type="button"
                variant="ghost"
                size="icon-sm"
                aria-label={`Remove ${name} ${range(rule)}`}
                disabled={removeHours.isPending && removeHours.variables.id === rule.id}
                onClick={() => removeHours.mutate(rule)}
              >
                <X />
              </Button>
            </div>
          ),
        )}
        {form === null ? null : (
          <HoursForm
            // A different rule is a different form, with its own starting values.
            key={form}
            weekday={weekday}
            rule={editing}
            onDone={() => setForm(null)}
          />
        )}
      </div>
      {form === null ? (
        <Button
          type="button"
          variant="ghost"
          size="sm"
          className="justify-self-start sm:justify-self-end"
          aria-label={`Add hours on ${name}`}
          onClick={() => setForm("add")}
        >
          <Plus />
          Add hours
        </Button>
      ) : null}
    </div>
  );
}

function AvailabilityPage() {
  const query = useQuery(availabilityQuery);
  const settings = useQuery(hostSettingsQuery);
  const timezone = settings.data && zoneName(settings.data.timezone);

  return (
    <>
      <PageHeader
        title="Availability"
        description={`The weekly hours when people can book a call with you. Pick a range to change it.${
          timezone ? ` Times are in ${timezone}.` : ""
        }`}
        actions={
          <Button variant="outline" size="sm" asChild>
            <Link to={ROUTES.dashboardSettings} hash={SETTINGS_SECTIONS.time}>
              Change timezone
            </Link>
          </Button>
        }
      />
      {query.isError ? (
        <ErrorState
          title="Your hours did not load"
          error={query.error}
          onRetry={() => void query.refetch()}
        />
      ) : (
        <Card className="gap-0 divide-y overflow-hidden py-0">
          {query.isPending ? (
            <TableSkeleton rows={7} />
          ) : (
            <>
              <WeekRuler week={weekHours(query.data)} />
              {WEEKDAYS.map((weekday) => (
                <DayRow
                  key={weekday}
                  weekday={weekday}
                  rules={query.data.filter((rule) => rule.weekday === weekday)}
                  week={weekHours(query.data)}
                />
              ))}
            </>
          )}
        </Card>
      )}
    </>
  );
}
