import { Clock, Globe, Video } from "lucide-react";
import type { ReactNode } from "react";

import type { BookingPage } from "../../bindings/BookingPage";
import { callPlace, CONFERENCING_APP } from "../integrations/conferencing";
import { GoogleMeetIcon } from "../integrations/google-meet-icon";
import { HostBadge } from "./host-badge";

/** Left column of the booking card: who the call is with, and where and when it happens. */
export function BookingSummary({
  page,
  minutes,
  timezone,
  children,
}: {
  page: BookingPage;
  /** Call length, once it is known. Hosts can offer different lengths on different days. */
  minutes: number | null;
  /** Timezone picker on the time step, plain label on the details step. */
  timezone: ReactNode;
  children?: ReactNode;
}) {
  const place = callPlace({
    app: page.conferencing_app,
    calendarConnected: page.calendar_connected,
    hasLink: page.has_meeting_link,
  });
  return (
    <div className="flex flex-col gap-4">
      <div>
        <HostBadge name={page.host_name} />
        <h1 className="mt-3 font-display text-2xl leading-tight font-semibold tracking-tight break-words">
          {page.title}
        </h1>
        {page.description ? (
          <p className="mt-2 text-sm text-muted-foreground">{page.description}</p>
        ) : null}
      </div>
      <ul className="flex flex-col gap-2 text-sm font-medium">
        {minutes === null ? null : (
          <li className="flex items-center gap-2">
            <Clock className="size-4 shrink-0 text-muted-foreground" />
            {minutes} minutes
          </li>
        )}
        {place === CONFERENCING_APP.googleMeet ? (
          <li className="flex items-center gap-2">
            <GoogleMeetIcon />
            Google Meet
          </li>
        ) : place === CONFERENCING_APP.customLink ? (
          <li className="flex items-center gap-2">
            <Video className="size-4 shrink-0 text-muted-foreground" />
            Video call
          </li>
        ) : null}
        <li className="flex items-center gap-2">
          <Globe className="size-4 shrink-0 text-muted-foreground" />
          {timezone}
        </li>
      </ul>
      {children}
    </div>
  );
}
