import { Link } from "@tanstack/react-router";
import {
  ArrowLeftRight,
  CalendarRange,
  Clock,
  EyeOff,
  ExternalLink,
  Hourglass,
  MessageCircleQuestion,
} from "lucide-react";
import type { ComponentType } from "react";
import { cn } from "cn";

import type { EventTypeInput } from "../../bindings/EventTypeInput";
import { Button } from "../../components/ui/button";
import { lengthLabel } from "../../lib/format";
import { ROUTES } from "../../lib/routes";
import { BOOKING_FRAME } from "../widget/booking-card";
import { QUESTIONS } from "../widget/booking-form";
import { HostBadge } from "../widget/host-badge";
import { accentStyle } from "../widget/public-booking-page";

function plural(count: number, one: string, many: string): string {
  return `${count} ${count === 1 ? one : many}`;
}

/** The booking rules in a visitor's words. A number being typed is left out. */
function ruleLines(values: EventTypeInput) {
  const lines: { icon: ComponentType<{ className?: string }>; text: string }[] = [];
  const notice = values.minimum_notice_hours;
  if (!Number.isNaN(notice)) {
    lines.push({
      icon: Hourglass,
      text:
        notice === 0
          ? "Bookable right up to the start"
          : `Booked at least ${plural(notice, "hour", "hours")} ahead`,
    });
  }
  const window = values.booking_window_days;
  if (!Number.isNaN(window)) {
    lines.push({
      icon: CalendarRange,
      text: `Times up to ${plural(window, "day", "days")} out`,
    });
  }
  const buffer = values.buffer_minutes;
  if (!Number.isNaN(buffer)) {
    lines.push({
      icon: ArrowLeftRight,
      text: buffer === 0 ? "Back to back allowed" : `${buffer} min kept free around it`,
    });
  }
  const asked = QUESTIONS.filter((question) => values[question.flag]).length;
  if (asked > 0) {
    lines.push({
      icon: MessageCircleQuestion,
      text: `Asks ${plural(asked, "more question", "more questions")}`,
    });
  }
  return lines;
}

/** The call as it reads on the host's link, following the form as it is edited. */
export function TypePreview({
  values,
  hostName,
  accent,
  hostSlug,
  savedSlug,
}: {
  values: EventTypeInput;
  hostName: string;
  /** The host's booking-page colour, so the preview wears it too. */
  accent: string;
  hostSlug: string;
  /** The link that works now; an edited one only works once saved. */
  savedSlug: string;
}) {
  return (
    <div className="grid gap-3">
      <h2 className="text-sm font-medium text-muted-foreground">Preview</h2>
      <div className={cn(BOOKING_FRAME, "host-accent")} style={accentStyle(accent)}>
        {values.is_active ? null : (
          <p className="flex items-center gap-2 border-b bg-muted/60 px-5 py-2.5 text-sm text-muted-foreground">
            <EyeOff className="size-4 shrink-0" />
            Not on your link while it is off.
          </p>
        )}
        <div className="grid gap-3 p-5">
          <HostBadge name={hostName} />
          <div>
            <h3 className="font-display text-xl leading-tight font-semibold tracking-tight break-words">
              {values.title.trim() || "Untitled call"}
            </h3>
            {values.description.trim() ? (
              <p className="mt-1.5 text-sm break-words text-muted-foreground">
                {values.description}
              </p>
            ) : null}
          </div>
          {values.duration_minutes === null ? null : (
            <p className="inline-flex w-fit items-center gap-1.5 rounded-full border px-2.5 py-0.5 text-xs font-medium text-muted-foreground">
              <Clock className="size-3.5" aria-hidden />
              {lengthLabel(values.duration_minutes)}
            </p>
          )}
        </div>
        <ul className="grid gap-2 border-t px-5 py-4 text-sm">
          {ruleLines(values).map((line) => (
            <li key={line.text} className="flex items-center gap-2.5">
              <line.icon className="size-4 shrink-0 text-muted-foreground" />
              {line.text}
            </li>
          ))}
        </ul>
      </div>
      <Button variant="outline" size="sm" className="w-fit" asChild>
        <Link
          to={ROUTES.bookingType}
          params={{ slug: hostSlug, type: savedSlug }}
          target="_blank"
        >
          <ExternalLink />
          Open booking page
        </Link>
      </Button>
    </div>
  );
}
