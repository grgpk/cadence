import { CalendarPlus, Download } from "lucide-react";

import { Button } from "../../components/ui/button";
import {
  type CalendarEvent,
  googleCalendarUrl,
  icsHref,
  outlookCalendarUrl,
} from "../../lib/calendar-links";

/** A booked call into whichever calendar the invitee keeps. */
export function AddToCalendar({ event }: { event: CalendarEvent }) {
  return (
    <>
      <Button variant="outline" asChild>
        <a href={googleCalendarUrl(event)} target="_blank" rel="noreferrer">
          <CalendarPlus />
          Add to Google Calendar
        </a>
      </Button>
      <Button variant="outline" asChild>
        <a href={outlookCalendarUrl(event)} target="_blank" rel="noreferrer">
          <CalendarPlus />
          Add to Outlook
        </a>
      </Button>
      <Button variant="outline" asChild>
        <a href={icsHref(event)} download="cadence-call.ics">
          <Download />
          Download .ics
        </a>
      </Button>
    </>
  );
}
