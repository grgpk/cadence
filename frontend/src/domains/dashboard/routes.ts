import {
  CalendarCheck,
  CalendarRange,
  Clock,
  CreditCard,
  LayoutDashboard,
  LayoutGrid,
  Settings,
} from "lucide-react";

import { ROUTES } from "../../lib/routes";
import type { NavSection } from "../shared/app-shell";

export const DASHBOARD_NAV: NavSection[] = [
  {
    items: [
      { title: "Overview", to: ROUTES.dashboard, icon: LayoutDashboard, exact: true },
      { title: "Bookings", to: ROUTES.dashboardBookings, icon: CalendarCheck },
      { title: "Availability", to: ROUTES.dashboardAvailability, icon: Clock },
      { title: "Event types", to: ROUTES.dashboardEventTypes, icon: CalendarRange },
      {
        title: "Apps",
        to: ROUTES.dashboardApps,
        icon: LayoutGrid,
        items: [
          { title: "App store", to: ROUTES.dashboardApps },
          { title: "Installed apps", to: ROUTES.dashboardAppsInstalled },
        ],
      },
      { title: "Settings", to: ROUTES.dashboardSettings, icon: Settings },
      { title: "Billing", to: ROUTES.dashboardBilling, icon: CreditCard },
    ],
  },
];

/** Anchors on the settings page, so other pages can link to one setting. */
export const SETTINGS_SECTIONS = {
  bookingPage: "booking-page",
  time: "time",
  appearance: "appearance",
  account: "account",
} as const;
