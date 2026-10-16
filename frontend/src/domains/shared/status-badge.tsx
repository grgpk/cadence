import { cn } from "cn";

import type { BookingStatus } from "../../bindings/BookingStatus";
import type { EmailState } from "../../bindings/EmailState";
import type { LeadQualification } from "../../bindings/LeadQualification";
import type { SubscriptionStatus } from "../../bindings/SubscriptionStatus";
import type { UserStatus } from "../../bindings/UserStatus";
import { Badge } from "../../components/ui/badge";

const TONES = {
  positive: "border-success/30 bg-success/10 text-success",
  caution: "border-warning/30 bg-warning/10 text-warning",
  negative: "border-destructive/30 bg-destructive/10 text-destructive",
  neutral: "border-border bg-muted text-muted-foreground",
} as const;

type Status = { label: string; tone: keyof typeof TONES };

export const BOOKING_STATUS: Record<BookingStatus, Status> = {
  confirmed: { label: "Confirmed", tone: "positive" },
  pending: { label: "Pending", tone: "caution" },
  cancelled: { label: "Cancelled", tone: "negative" },
};

export const LEAD_STATUS: Record<LeadQualification, Status> = {
  Qualified: { label: "Qualified", tone: "positive" },
  NotSure: { label: "Not sure", tone: "caution" },
  Disqualified: { label: "Disqualified", tone: "negative" },
};

/** An email that will not send keeps retrying, then is given up on as failed. */
export const EMAIL_STATUS: Record<EmailState, Status> = {
  Sent: { label: "Sent", tone: "positive" },
  Scheduled: { label: "Scheduled", tone: "neutral" },
  Retrying: { label: "Retrying", tone: "caution" },
  Failed: { label: "Failed", tone: "negative" },
  Overdue: { label: "Overdue", tone: "caution" },
};

export const USER_STATUS: Record<UserStatus, Status> = {
  Active: { label: "Active", tone: "positive" },
  Banned: { label: "Banned", tone: "negative" },
};

export const SUBSCRIPTION_STATUS: Record<SubscriptionStatus, Status> = {
  trialing: { label: "Trial", tone: "neutral" },
  active: { label: "Active", tone: "positive" },
  past_due: { label: "Payment failed", tone: "negative" },
  canceled: { label: "Ended", tone: "neutral" },
  incomplete: { label: "Not paid", tone: "caution" },
  comped: { label: "Comped", tone: "positive" },
};

export function StatusBadge<T extends string>({
  value,
  statuses,
  fallback = "Unknown",
}: {
  value: T | null;
  statuses: Partial<Record<T, Status>>;
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
