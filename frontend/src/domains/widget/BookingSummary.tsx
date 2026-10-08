import { Clock, Globe, Video } from "lucide-react";
import type { ReactNode } from "react";

import type { BookingPage } from "../../bindings/BookingPage";

function GoogleMeetIcon() {
  return (
    <svg
      viewBox="0 0 22 22"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
      aria-hidden="true"
      className="size-4 shrink-0"
    >
      <path
        d="M1.375 7.79159V14.2083L3.66667 14.6666L5.95833 14.2083V7.79159L3.66667 7.33325L1.375 7.79159Z"
        fill="#1E88E5"
      />
      <path
        d="M16.9583 11.0001V17.4167C16.9583 18.1762 16.3428 18.7917 15.5833 18.7917H5.95833L5.5 16.5001L5.95833 14.2084H12.375V11.0001L14.6667 10.5417L16.9583 11.0001Z"
        fill="#4CAF50"
      />
      <path
        d="M16.9583 4.58325V10.9999H12.375V7.79158H5.95833L5.5 5.49992L5.95833 3.20825H15.5833C16.3428 3.20825 16.9583 3.82379 16.9583 4.58325Z"
        fill="#FBC02D"
      />
      <path
        d="M5.95833 14.2083V18.7916H2.75C1.99054 18.7916 1.375 18.176 1.375 17.4166V14.2083H5.95833Z"
        fill="#1565C0"
      />
      <path d="M5.95833 3.20825V7.79159H1.375L5.95833 3.20825Z" fill="#E53935" />
      <path
        d="M17.4167 11.0001L16.9583 14.873L12.375 11.0001L16.9583 7.1272L17.4167 11.0001Z"
        fill="#2E7D32"
      />
      <path
        d="M21.082 4.63383V17.3663C21.082 17.7513 20.6329 17.9667 20.3349 17.7238L16.957 14.873V7.12716L20.3349 4.27633C20.6329 4.03341 21.082 4.24883 21.082 4.63383Z"
        fill="#4CAF50"
      />
    </svg>
  );
}

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
  return (
    <div className="flex flex-col gap-4">
      <div>
        <p className="text-sm font-medium text-muted-foreground">{page.host_name}</p>
        <h1 className="mt-1 font-display text-2xl leading-tight font-semibold tracking-tight">
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
        {page.calendar_connected ? (
          <li className="flex items-center gap-2">
            <GoogleMeetIcon />
            Google Meet
          </li>
        ) : page.has_meeting_link ? (
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
