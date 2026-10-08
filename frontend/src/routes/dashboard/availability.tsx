import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { createFileRoute, Link } from "@tanstack/react-router";
import { Plus, X } from "lucide-react";
import { useState } from "react";
import { toast } from "sonner";

import type { AvailabilityRule } from "../../bindings/AvailabilityRule";
import type { AvailabilityRuleInput } from "../../bindings/AvailabilityRuleInput";
import { Button } from "../../components/ui/button";
import { Card } from "../../components/ui/card";
import { Input } from "../../components/ui/input";
import { Label } from "../../components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "../../components/ui/select";
import { ErrorState } from "../../domains/shared/ErrorState";
import { PageHeader } from "../../domains/shared/PageHeader";
import { TableSkeleton } from "../../domains/shared/TableSkeleton";
import { getHostSettings } from "../../domains/widget/cadence-api";
import { api } from "../../lib/api";
import { APP_NAME } from "../../lib/config";
import { formatClock, weekdayName } from "../../lib/format";
import { QUERY_KEYS } from "../../lib/query-keys";
import { ROUTES } from "../../lib/routes";

export const Route = createFileRoute("/dashboard/availability")({
  head: () => ({ meta: [{ title: `Availability | ${APP_NAME}` }] }),
  component: AvailabilityPage,
});

const WEEKDAYS = [0, 1, 2, 3, 4, 5, 6];
const SLOT_MINUTES = [15, 20, 30, 45, 60];
const DEFAULT_HOURS = { start: "09:00", end: "17:00", slotMinutes: 30 };

function errorMessage(error: unknown) {
  return error instanceof Error ? error.message : "Something went wrong.";
}

function AddHoursForm({ weekday, onDone }: { weekday: number; onDone: () => void }) {
  const queryClient = useQueryClient();
  const [start, setStart] = useState(DEFAULT_HOURS.start);
  const [end, setEnd] = useState(DEFAULT_HOURS.end);
  const [slotMinutes, setSlotMinutes] = useState(DEFAULT_HOURS.slotMinutes);
  const endsAfterStart = start < end;
  const id = `hours-${weekday}`;

  const addHours = useMutation({
    mutationFn: (input: AvailabilityRuleInput) =>
      api<AvailabilityRule>("/api/availability", {
        method: "POST",
        body: JSON.stringify(input),
      }),
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: QUERY_KEYS.availability });
      toast.success("Hours added");
      onDone();
    },
    onError: (error) => toast.error(`Hours were not added. ${errorMessage(error)}`),
  });

  return (
    <form
      className="grid gap-3 rounded-lg border bg-muted p-3"
      onSubmit={(event) => {
        event.preventDefault();
        if (!endsAfterStart) return;
        addHours.mutate({
          weekday,
          start_time: `${start}:00`,
          end_time: `${end}:00`,
          slot_minutes: slotMinutes,
        });
      }}
    >
      <div className="flex flex-wrap items-end gap-3">
        <div className="grid gap-1.5">
          <Label htmlFor={`${id}-start`}>From</Label>
          <Input
            id={`${id}-start`}
            type="time"
            required
            value={start}
            onChange={(event) => setStart(event.target.value)}
            className="w-28 bg-card tabular-nums"
          />
        </div>
        <div className="grid gap-1.5">
          <Label htmlFor={`${id}-end`}>To</Label>
          <Input
            id={`${id}-end`}
            type="time"
            required
            value={end}
            aria-invalid={!endsAfterStart}
            onChange={(event) => setEnd(event.target.value)}
            className="w-28 bg-card tabular-nums"
          />
        </div>
        <div className="grid gap-1.5">
          <Label htmlFor={`${id}-length`}>Call length</Label>
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
        </div>
        <div className="ml-auto flex gap-2">
          <Button type="button" variant="ghost" onClick={onDone}>
            Cancel
          </Button>
          <Button type="submit" disabled={addHours.isPending || !endsAfterStart}>
            {addHours.isPending ? "Adding" : "Add hours"}
          </Button>
        </div>
      </div>
      {endsAfterStart ? null : (
        <p className="text-sm text-destructive">The end time must be after the start.</p>
      )}
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
            key={String(rule.id)}
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

  const removeHours = useMutation({
    mutationFn: (rule: AvailabilityRule) =>
      api<void>(`/api/availability/${rule.id}`, { method: "DELETE" }),
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: QUERY_KEYS.availability });
      toast.success("Hours removed");
    },
    onError: (error) => toast.error(`Hours were not removed. ${errorMessage(error)}`),
  });

  return (
    <div className="grid gap-3 px-5 py-4 sm:grid-cols-[7rem_minmax(0,1fr)_auto] sm:items-start">
      <h2 className="pt-1.5 text-sm font-semibold">{name}</h2>
      <div className="grid gap-2">
        <DayStrip rules={rules} />
        {rules.length === 0 && !adding ? (
          <p className="text-sm text-muted-foreground">Unavailable</p>
        ) : null}
        {rules.map((rule) => {
          const range = `${formatClock(rule.start_time, false)} to ${formatClock(rule.end_time, false)}`;
          return (
            <div key={String(rule.id)} className="flex items-center gap-3 text-sm">
              <span className="rounded-md border bg-card px-2.5 py-1 font-medium tabular-nums">
                {range}
              </span>
              <span className="text-muted-foreground">{rule.slot_minutes} min calls</span>
              <Button
                type="button"
                variant="ghost"
                size="icon-sm"
                aria-label={`Remove ${name} ${range}`}
                disabled={removeHours.isPending}
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
  const query = useQuery({
    queryKey: QUERY_KEYS.availability,
    queryFn: () => api<AvailabilityRule[]>("/api/availability"),
  });
  const settings = useQuery({
    queryKey: QUERY_KEYS.hostSettings,
    queryFn: getHostSettings,
  });
  const timezone = settings.data?.timezone.replaceAll("_", " ");

  return (
    <>
      <PageHeader
        title="Availability"
        description={`The weekly hours when people can book a call with you.${
          timezone ? ` Times are in ${timezone}.` : ""
        }`}
        actions={
          <Button variant="outline" size="sm" asChild>
            <Link to={ROUTES.dashboardSettings}>Change timezone</Link>
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
