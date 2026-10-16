import { CalendarDays, Video, type LucideIcon } from "lucide-react";

/** An app hosts will be able to connect, shown before it works. */
export type ComingSoonApp = {
  name: string;
  description: string;
  icon: LucideIcon;
};

/** Calendars that will keep busy times off the booking page. */
export const COMING_SOON_CALENDARS: readonly ComingSoonApp[] = [
  {
    name: "Outlook Calendar",
    description: "Microsoft 365 and Outlook.com calendars.",
    icon: CalendarDays,
  },
  {
    name: "Apple Calendar",
    description: "iCloud and other CalDAV calendars.",
    icon: CalendarDays,
  },
];

/** Meeting apps that will make a link for each call. */
export const COMING_SOON_CONFERENCING: readonly ComingSoonApp[] = [
  { name: "Zoom", description: "A new Zoom meeting for each call.", icon: Video },
  {
    name: "Microsoft Teams",
    description: "A new Teams meeting for each call.",
    icon: Video,
  },
];
