import { CalendarCheck, Clock, LayoutDashboard, Settings } from "lucide-react";

import { ROUTES } from "../../lib/routes";
import type { NavSection } from "../shared/AppShell";

export const DASHBOARD_NAV: NavSection[] = [
  {
    items: [
      { title: "Overview", to: ROUTES.dashboard, icon: LayoutDashboard, exact: true },
      { title: "Bookings", to: ROUTES.dashboardBookings, icon: CalendarCheck },
      { title: "Availability", to: ROUTES.dashboardAvailability, icon: Clock },
      { title: "Settings", to: ROUTES.dashboardSettings, icon: Settings },
    ],
  },
];

/** Anchors on the settings page, so other pages can link to one setting. */
export const SETTINGS_SECTIONS = {
  bookingPage: "booking-page",
  time: "time",
  calls: "calls",
  questions: "questions",
} as const;
