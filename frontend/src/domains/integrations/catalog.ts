import { CalendarDays, Link as LinkIcon, Video } from "lucide-react";
import type { ComponentType } from "react";

import type { HostSettings } from "../../bindings/HostSettings";
import { CONFERENCING_APP } from "./conferencing";
import { GoogleCalendarIcon } from "./google-calendar-icon";
import { GoogleMeetIcon } from "./google-meet-icon";

export const APP_CATEGORY = {
  calendar: "calendar",
  conferencing: "conferencing",
} as const;

export type AppCategory = (typeof APP_CATEGORY)[keyof typeof APP_CATEGORY];

export const CATEGORY_INFO: Record<
  AppCategory,
  { name: string; description: string; icon: ComponentType<{ className?: string }> }
> = {
  [APP_CATEGORY.calendar]: {
    name: "Calendar",
    description: "Keep busy times off your booking page.",
    icon: CalendarDays,
  },
  [APP_CATEGORY.conferencing]: {
    name: "Conferencing",
    description: "Where your calls happen.",
    icon: Video,
  },
};

export const APP_SLUG = {
  googleCalendar: "google-calendar",
  googleMeet: "google-meet",
  customLink: "custom-link",
  outlook: "outlook",
  appleCalendar: "apple-calendar",
  zoom: "zoom",
  teams: "teams",
} as const;

export type AppSlug = (typeof APP_SLUG)[keyof typeof APP_SLUG];

export type AppEntry = {
  slug: AppSlug;
  name: string;
  category: AppCategory;
  /** One line for the store card. */
  summary: string;
  /** What the app does here, on its own page. */
  description: string;
  icon: ComponentType<{ className?: string }>;
  /** `false` while the app is on the way: listed, but nothing to set up. */
  available: boolean;
  /** Another app this one only works with. */
  requires?: AppSlug;
};

/** Every app a host can connect, or will be able to soon. */
export const APPS: readonly AppEntry[] = [
  {
    slug: APP_SLUG.googleCalendar,
    name: "Google Calendar",
    category: APP_CATEGORY.calendar,
    summary:
      "Busy times stay off your booking page, and each call lands on your calendar.",
    description:
      "Connect Google Calendar so nobody can book over your other plans. Every new call is added to your calendar, and Google Meet links are made from it.",
    icon: GoogleCalendarIcon,
    available: true,
  },
  {
    slug: APP_SLUG.googleMeet,
    name: "Google Meet",
    category: APP_CATEGORY.conferencing,
    summary: "A new Meet link for each call, added to the calendar event.",
    description:
      "Each booked call gets its own Google Meet link, sent in the confirmation and added to the calendar event.",
    icon: GoogleMeetIcon,
    available: true,
    requires: APP_SLUG.googleCalendar,
  },
  {
    slug: APP_SLUG.customLink,
    name: "Custom link",
    category: APP_CATEGORY.conferencing,
    summary: "One Zoom, Teams or other room for every call.",
    description:
      "Use one meeting room for every call, such as a personal Zoom or Teams link. Everyone who books gets it in their confirmation.",
    icon: LinkIcon,
    available: true,
  },
  {
    slug: APP_SLUG.outlook,
    name: "Outlook Calendar",
    category: APP_CATEGORY.calendar,
    summary: "Microsoft 365 and Outlook.com calendars.",
    description:
      "Keep busy times from Microsoft 365 and Outlook.com off your booking page.",
    icon: CalendarDays,
    available: false,
  },
  {
    slug: APP_SLUG.appleCalendar,
    name: "Apple Calendar",
    category: APP_CATEGORY.calendar,
    summary: "iCloud and other CalDAV calendars.",
    description:
      "Keep busy times from iCloud and other CalDAV calendars off your booking page.",
    icon: CalendarDays,
    available: false,
  },
  {
    slug: APP_SLUG.zoom,
    name: "Zoom",
    category: APP_CATEGORY.conferencing,
    summary: "A new Zoom meeting for each call.",
    description: "Each booked call gets its own Zoom meeting.",
    icon: Video,
    available: false,
  },
  {
    slug: APP_SLUG.teams,
    name: "Microsoft Teams",
    category: APP_CATEGORY.conferencing,
    summary: "A new Teams meeting for each call.",
    description: "Each booked call gets its own Microsoft Teams meeting.",
    icon: Video,
    available: false,
  },
];

export function findApp(slug: string): AppEntry | undefined {
  return APPS.find((app) => app.slug === slug);
}

/** Whether the host has an app set up and in use. */
export function isInstalled(
  slug: AppSlug,
  settings: Pick<HostSettings, "conferencing_app" | "meeting_link">,
  calendarConnected: boolean,
): boolean {
  switch (slug) {
    case APP_SLUG.googleCalendar:
      return calendarConnected;
    case APP_SLUG.googleMeet:
      return settings.conferencing_app === CONFERENCING_APP.googleMeet;
    case APP_SLUG.customLink:
      return (
        settings.conferencing_app === CONFERENCING_APP.customLink &&
        Boolean(settings.meeting_link)
      );
    default:
      return false;
  }
}

export const APP_TONE = { on: "on", warn: "warn", off: "off" } as const;
export type AppTone = (typeof APP_TONE)[keyof typeof APP_TONE];

/** "zoom.us/j/123" for "https://zoom.us/j/123": the part a host recognises. */
function shortLink(link: string): string {
  return link.replace(/^https?:\/\//, "").replace(/\/$/, "");
}

/** What an app is doing for the host right now, in a few words. */
export function appState(
  slug: AppSlug,
  settings: Pick<HostSettings, "conferencing_app" | "meeting_link">,
  calendarConnected: boolean,
): { tone: AppTone; text: string } {
  if (!isInstalled(slug, settings, calendarConnected)) {
    return { tone: APP_TONE.off, text: "Not set up" };
  }
  switch (slug) {
    case APP_SLUG.googleCalendar:
      return { tone: APP_TONE.on, text: "Busy times blocked" };
    case APP_SLUG.googleMeet:
      // The same rule as `callPlace`: Meet needs a calendar to make its links.
      return calendarConnected
        ? { tone: APP_TONE.on, text: "New link for each call" }
        : { tone: APP_TONE.warn, text: "Needs Google Calendar" };
    default:
      return { tone: APP_TONE.on, text: shortLink(settings.meeting_link ?? "") };
  }
}

/** Apps whose name or summary contains `search`, in `category` when one is picked. */
export function filterApps(
  apps: readonly AppEntry[],
  search: string,
  category: AppCategory | undefined,
): AppEntry[] {
  const wanted = search.trim().toLowerCase();
  return apps.filter(
    (app) =>
      (category === undefined || app.category === category) &&
      (wanted === "" ||
        app.name.toLowerCase().includes(wanted) ||
        app.summary.toLowerCase().includes(wanted)),
  );
}
