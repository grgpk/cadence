import { cn } from "cn";

import { Badge } from "../../components/ui/badge";

const TONES = {
  positive: "border-success/30 bg-success/10 text-success",
  caution: "border-warning/30 bg-warning/10 text-warning",
  negative: "border-destructive/30 bg-destructive/10 text-destructive",
  neutral: "border-border bg-muted text-muted-foreground",
} as const;

type Status = { label: string; tone: keyof typeof TONES };

/** Booking statuses, as constrained by the `bookings_status` check. */
export const BOOKING_STATUS: Record<string, Status> = {
  confirmed: { label: "Confirmed", tone: "positive" },
  pending: { label: "Pending", tone: "caution" },
  cancelled: { label: "Cancelled", tone: "negative" },
};

/** Lead qualification, as constrained by the `leads_qualification_status` check. */
export const LEAD_STATUS: Record<string, Status> = {
  Qualified: { label: "Qualified", tone: "positive" },
  NotSure: { label: "Not sure", tone: "caution" },
  Disqualified: { label: "Disqualified", tone: "negative" },
};

export function StatusBadge({
  value,
  statuses,
  fallback = "Unknown",
}: {
  value: string | null;
  statuses: Record<string, Status>;
  fallback?: string;
}) {
  const status = (value ? statuses[value] : undefined) ?? {
    label: value ?? fallback,
    tone: "neutral" as const,
  };
  return (
    <Badge variant="outline" className={cn(TONES[status.tone])}>
      {status.label}
    </Badge>
  );
}
