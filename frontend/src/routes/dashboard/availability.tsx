import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { createFileRoute, Link } from "@tanstack/react-router";
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
} from "../../domains/dashboard/host-api";
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

function AddHoursForm({ weekday, onDone }: { weekday: number; onDone: () => void }) {
  const queryClient = useQueryClient();
  const [start, setStart] = useState(DEFAULT_HOURS.start);
  const [end, setEnd] = useState(DEFAULT_HOURS.end);
  const [slotMinutes, setSlotMinutes] = useState(DEFAULT_HOURS.slotMinutes);
  const [otherDays, setOtherDays] = useState<number[]>([]);
  const endsAfterStart = start < end;
  const id = `hours-${weekday}`;

  const addHours = useMutation({
    mutationFn: (days: number[]) =>
      Promise.all(
        days.map((day) =>
          createAvailabilityRule({
            weekday: day,
            start_time: `${start}:00`,
            end_time: `${end}:00`,
            slot_minutes: slotMinutes,
          }),
        ),
      ),
    onSuccess: () => {
      toast.success("Hours added");
      onDone();
    },
    onError: (error) =>
      toast.error(
        otherDays.length === 0
          ? `Hours were not added. ${errorMessage(error)}`
          : `Some days were not added. ${errorMessage(error)}`,
      ),
    // Some days may have been saved even when one failed.
    onSettled: () => queryClient.invalidateQueries({ queryKey: QUERY_KEYS.availability }),
  });

  return (
    <form
      className="grid gap-3 rounded-lg border bg-muted p-3"
      onSubmit={(event) => {
        event.preventDefault();
        if (endsAfterStart) addHours.mutate([weekday, ...otherDays]);
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
        <Button type="submit" disabled={addHours.isPending || !endsAfterStart}>
          {addHours.isPending ? <Loader2 className="animate-spin" /> : null}
          {addHours.isPending ? "Adding" : "Add hours"}
        </Button>
      </div>
    </form>
  );
}

const DAY_SECONDS = 24 * 60 * 60;

/** Seconds since midnight for a wall-clock "HH:MM:SS". */
function secondsOfDay(time: string): number {
  const [hours = 0, minutes = 0, seconds = 0] = time.split(":").map(Number);
  return hours * 3600 + minutes * 60 + seconds;
}

/** The day as a strip from midnight to midnight, with the open hours marked. */
function DayStrip({ rules }: { rules: AvailabilityRule[] }) {
  return (
    <div
      aria-hidden="true"
      className="relative h-1.5 overflow-hidden rounded-full bg-muted"
    >
      {rules.map((rule) => {
        const start = secondsOfDay(rule.start_time);
        const end = secondsOfDay(rule.end_time);
        return (
          <span
            key={rule.id}
            className="absolute inset-y-0 rounded-full bg-noon"
            style={{
              left: `${(start / DAY_SECONDS) * 100}%`,
              width: `${((end - start) / DAY_SECONDS) * 100}%`,
            }}
          />
        );
      })}
    </div>
  );
}

function DayRow({ weekday, rules }: { weekday: number; rules: AvailabilityRule[] }) {
  const queryClient = useQueryClient();
  const [adding, setAdding] = useState(false);
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

  return (
    <div className="grid gap-3 px-5 py-4 sm:grid-cols-[7rem_minmax(0,1fr)_auto] sm:items-start">
      <h2 className="pt-1.5 text-sm font-semibold">{name}</h2>
      <div className="grid gap-2">
        <DayStrip rules={rules} />
        {rules.length === 0 && !adding ? (
          <p className="text-sm text-muted-foreground">Unavailable</p>
        ) : null}
        {rules.map((rule) => {
          const range = `${formatClock(rule.start_time, hour12)} to ${formatClock(rule.end_time, hour12)}`;
          return (
            <div key={rule.id} className="flex items-center gap-3 text-sm">
              <span className="rounded-md border bg-card px-2.5 py-1 font-medium tabular-nums">
                {range}
              </span>
              <span className="text-muted-foreground">{rule.slot_minutes} min calls</span>
              <Button
                type="button"
                variant="ghost"
                size="icon-sm"
                aria-label={`Remove ${name} ${range}`}
                disabled={removeHours.isPending && removeHours.variables.id === rule.id}
                onClick={() => removeHours.mutate(rule)}
              >
                <X />
              </Button>
            </div>
          );
        })}
        {adding ? (
          <AddHoursForm weekday={weekday} onDone={() => setAdding(false)} />
        ) : null}
      </div>
      {adding ? null : (
        <Button
          type="button"
          variant="ghost"
          size="sm"
          className="justify-self-start"
          aria-label={`Add hours on ${name}`}
          onClick={() => setAdding(true)}
        >
          <Plus />
          Add hours
        </Button>
      )}
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
        description={`The weekly hours when people can book a call with you.${
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
            WEEKDAYS.map((weekday) => (
              <DayRow
                key={weekday}
                weekday={weekday}
                rules={query.data.filter((rule) => rule.weekday === weekday)}
              />
            ))
          )}
        </Card>
      )}
    </>
  );
}
