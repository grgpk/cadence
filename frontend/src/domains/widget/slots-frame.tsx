import { useQuery } from "@tanstack/react-query";
import { CalendarDays } from "lucide-react";
import type { ReactNode } from "react";

import { Skeleton } from "../../components/ui/skeleton";
import { ApiError } from "../../lib/api";
import { EmptyState } from "../shared/empty-state";
import { ErrorState } from "../shared/error-state";
import { SegmentedControl } from "../shared/segmented-control";
import { publicSlotsQuery } from "./cadence-api";
import { slotsByDay, type SlotDay } from "./slot-days";
import { CLOCKS } from "./slot-step";

const LOADING_ROWS = [0, 1, 2, 3];

/**
 * The column the week and list views sit in: a heading with the clock choice,
 * then the host's open times by day, or why there are none.
 */
export function SlotsFrame({
  slug,
  timeZone,
  title,
  controls,
  emptyDescription,
  hour12,
  onHour12Change,
  children,
}: {
  slug: string;
  timeZone: string;
  title: (days: SlotDay[]) => ReactNode;
  /** Shown beside the title, before the clock choice. */
  controls?: (days: SlotDay[]) => ReactNode;
  emptyDescription: string;
  hour12: boolean;
  onHour12Change: (hour12: boolean) => void;
  children: (days: SlotDay[]) => ReactNode;
}) {
  // The same request and cache as the month view, so switching views is instant.
  const query = useQuery(publicSlotsQuery(slug));
  const days = slotsByDay(query.data ?? [], timeZone);

  let body: ReactNode;
  if (query.isPending) {
    body = (
      <div className="grid gap-2" role="status" aria-label="Loading times">
        {LOADING_ROWS.map((row) => (
          <Skeleton key={row} className="h-10 w-full" />
        ))}
      </div>
    );
  } else if (
    (query.error instanceof ApiError && query.error.isNotFound) ||
    (!query.isError && days.length === 0)
  ) {
    body = (
      <EmptyState
        icon={CalendarDays}
        title="No open times"
        description={emptyDescription}
      />
    );
  } else if (query.isError) {
    body = (
      <ErrorState
        title="The times did not load"
        error={query.error}
        onRetry={() => void query.refetch()}
      />
    );
  } else {
    body = children(days);
  }

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
          options={CLOCKS}
          onChange={onHour12Change}
        />
      </div>
      {body}
    </div>
  );
}
