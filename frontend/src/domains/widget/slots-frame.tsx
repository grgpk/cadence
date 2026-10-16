import { type UseQueryResult, useQuery } from "@tanstack/react-query";
import { cn } from "cn";
import { CalendarDays } from "lucide-react";
import type { ReactNode } from "react";

import type { TimeSlot } from "../../bindings/TimeSlot";
import { Skeleton } from "../../components/ui/skeleton";
import { CLOCK_OPTIONS } from "../../hooks/use-clock-preference";
import { ApiError } from "../../lib/api";
import { EmptyState } from "../shared/empty-state";
import { ErrorState } from "../shared/error-state";
import { SegmentedControl } from "../shared/segmented-control";
import { publicSlotsQuery } from "./cadence-api";
import { slotsByDay, type SlotDay } from "./slot-days";

const LOADING_ROWS = [0, 1, 2, 3];

/** What to say when there is nothing to pick, and why. */
export type SlotsWording = {
  /** Said when the host has no open times at all. */
  emptyDescription: string;
  /** Said when the host's page stopped taking bookings; defaults to the above. */
  pausedDescription?: string;
};

/**
 * Loading, failed, paused or empty, in every slot view the same way. `null`
 * when there are times to show. `rowClassName` sizes the loading rows like
 * the view's own times.
 */
export function slotsStatus(
  query: UseQueryResult<TimeSlot[]>,
  { emptyDescription, pausedDescription }: SlotsWording,
  rowClassName: string,
): ReactNode {
  if (query.isPending) {
    return (
      <div className="grid gap-2" role="status" aria-label="Loading times">
        {LOADING_ROWS.map((row) => (
          <Skeleton key={row} className={cn("w-full", rowClassName)} />
        ))}
      </div>
    );
  }
  // The page was paused after this call was booked.
  if (query.error instanceof ApiError && query.error.isNotFound) {
    return (
      <EmptyState
        icon={CalendarDays}
        title="No open times"
        description={pausedDescription ?? emptyDescription}
      />
    );
  }
  if (query.isError) {
    return (
      <ErrorState
        title="The times did not load"
        error={query.error}
        onRetry={() => void query.refetch()}
      />
    );
  }
  if (query.data.length === 0) {
    return (
      <EmptyState
        icon={CalendarDays}
        title="No open times"
        description={emptyDescription}
      />
    );
  }
  return null;
}

/**
 * The column the week and list views sit in: a heading with the clock choice,
 * then the host's open times by day, or why there are none.
 */
export function SlotsFrame({
  slug,
  timeZone,
  title,
  controls,
  wording,
  hour12,
  onHour12Change,
  children,
}: {
  slug: string;
  timeZone: string;
  title: (days: SlotDay[]) => ReactNode;
  /** Shown beside the title, before the clock choice. */
  controls?: (days: SlotDay[]) => ReactNode;
  wording: SlotsWording;
  hour12: boolean;
  onHour12Change: (hour12: boolean) => void;
  children: (days: SlotDay[]) => ReactNode;
}) {
  // The same request and cache as the month view, so switching views is instant.
  const query = useQuery(publicSlotsQuery(slug));
  const days = slotsByDay(query.data ?? [], timeZone);

  const body = slotsStatus(query, wording, "h-10") ?? children(days);

  return (
    <div className="flex min-w-0 flex-col gap-4 p-5 @[760px]:p-6">
      <div className="flex flex-wrap items-center gap-x-3 gap-y-2">
        <h2
          aria-live="polite"
          className="mr-auto font-display text-lg font-semibold tracking-tight"
        >
          {title(days)}
        </h2>
        {days.length > 0 ? controls?.(days) : null}
        <SegmentedControl
          label="Clock"
          size="sm"
          value={hour12}
          options={CLOCK_OPTIONS}
          onChange={onHour12Change}
        />
      </div>
      {body}
    </div>
  );
}
