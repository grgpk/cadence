export type CalendarEvent = {
  title: string;
  start: Date;
  end: Date;
  /** Where to join, when there is a link. */
  location?: string;
  /** Stable per event, so importing it twice updates it instead of doubling it. */
  uid: string;
};

/** "20261012T080000Z", the form both Google and iCalendar read. */
function stamp(instant: Date): string {
  return instant.toISOString().replaceAll(/[-:]|\.\d{3}/g, "");
}

/** A link that opens Google Calendar with the event filled in. */
export function googleCalendarUrl(event: CalendarEvent): string {
  const params = new URLSearchParams({
    action: "TEMPLATE",
    text: event.title,
    dates: `${stamp(event.start)}/${stamp(event.end)}`,
  });
  if (event.location) params.set("location", event.location);
  return `https://calendar.google.com/calendar/render?${params}`;
}

function icsText(value: string): string {
  return value.replaceAll(/[\\;,]/g, "\\$&").replaceAll(/\r?\n/g, "\\n");
}

/** The event as an .ics file, for Apple Calendar, Outlook and the rest. */
export function icsFile(event: CalendarEvent): string {
  return [
    "BEGIN:VCALENDAR",
    "VERSION:2.0",
    "PRODID:-//Cadence//Booking//EN",
    "BEGIN:VEVENT",
    `UID:${icsText(event.uid)}`,
    // The file is built on every render, so the stamp cannot be "now".
    `DTSTAMP:${stamp(event.start)}`,
    `DTSTART:${stamp(event.start)}`,
    `DTEND:${stamp(event.end)}`,
    `SUMMARY:${icsText(event.title)}`,
    ...(event.location ? [`LOCATION:${icsText(event.location)}`] : []),
    "END:VEVENT",
    "END:VCALENDAR",
    "",
  ].join("\r\n");
}

/** The file as a link target, so a plain `<a download>` saves it. */
export function icsHref(event: CalendarEvent): string {
  return `data:text/calendar;charset=utf-8,${encodeURIComponent(icsFile(event))}`;
}
