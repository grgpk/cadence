import { cn } from "cn";
import { AlertTriangle, CalendarOff, Link as LinkIcon } from "lucide-react";
import type { ReactNode } from "react";

import type { ConferencingApp } from "../../bindings/ConferencingApp";
import { CONFERENCING_APP } from "./conferencing";
import { GoogleCalendarIcon } from "./google-calendar-icon";
import { GoogleMeetIcon } from "./google-meet-icon";

function Node({
  icon,
  title,
  detail,
  tone = "default",
  align = "start",
}: {
  icon: ReactNode;
  title: string;
  detail: string;
  tone?: "default" | "muted" | "warning";
  align?: "start" | "end";
}) {
  return (
    <div
      className={cn(
        "flex min-w-0 items-center gap-3",
        align === "end" && "sm:flex-row-reverse sm:text-right",
      )}
    >
      <span
        className={cn(
          "grid size-10 shrink-0 place-items-center rounded-lg border bg-card shadow-xs",
          tone === "muted" && "border-dashed bg-muted shadow-none",
          tone === "warning" && "border-warning/40 bg-warning/10",
        )}
      >
        {icon}
      </span>
      <div className="min-w-0">
        <p
          className={cn(
            "truncate text-sm font-semibold",
            tone === "muted" && "text-muted-foreground",
          )}
        >
          {title}
        </p>
        <p
          className={cn(
            "text-sm text-muted-foreground",
            tone === "warning" && "text-warning",
          )}
        >
          {detail}
        </p>
      </div>
    </div>
  );
}

const PLACES = {
  [CONFERENCING_APP.googleMeet]: {
    icon: <GoogleMeetIcon />,
    title: "Google Meet",
    detail: "A new link for each call",
  },
  [CONFERENCING_APP.customLink]: {
    icon: <LinkIcon className="size-4 text-muted-foreground" />,
    title: "Your own link",
    detail: "The same room for every call",
  },
} as const;

/**
 * What happens when someone books: the calendar the call goes on, joined to
 * where it takes place. The rail is drawn only when both ends are live.
 */
export function CallFlow({
  calendarConnected,
  place,
}: {
  calendarConnected: boolean;
  place: ConferencingApp | null;
}) {
  const live = calendarConnected && place !== null;
  const where = place === null ? null : PLACES[place];

  return (
    <div className="rounded-xl border bg-card p-5">
      <p className="mb-4 text-sm text-muted-foreground">When someone books you</p>
      <div className="grid items-center gap-3 sm:grid-cols-[minmax(0,1fr)_minmax(3rem,8rem)_minmax(0,1fr)] sm:gap-4">
        {calendarConnected ? (
          <Node
            icon={<GoogleCalendarIcon />}
            title="Google Calendar"
            detail="Your busy times stay blocked"
          />
        ) : (
          <Node
            icon={<CalendarOff className="size-4 text-muted-foreground" />}
            title="No calendar"
            detail="Busy times are not checked"
            tone="muted"
          />
        )}

        {/* Re-keyed so the rail draws again whenever both ends come alive. */}
        <div
          key={live ? "live" : "idle"}
          aria-hidden="true"
          className="ml-5 h-5 w-px sm:ml-0 sm:h-px sm:w-full"
        >
          {live ? (
            <span className="block size-full origin-top bg-foreground motion-safe:animate-bar-grow-y sm:origin-left sm:motion-safe:animate-bar-grow" />
          ) : (
            <span className="block size-full border-l border-dashed border-muted-foreground/50 sm:border-t sm:border-l-0" />
          )}
        </div>

        {where ? (
          <Node icon={where.icon} title={where.title} detail={where.detail} align="end" />
        ) : (
          <Node
            icon={<AlertTriangle className="size-4 text-warning" />}
            title="No meeting link"
            detail="People who book get no link to join"
            tone="warning"
            align="end"
          />
        )}
      </div>
    </div>
  );
}
