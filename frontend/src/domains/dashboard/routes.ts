import { CalendarCheck, Clock, LayoutDashboard } from "lucide-react";

import { ROUTES } from "../../lib/routes";
import type { NavSection } from "../shared/AppShell";

export const DASHBOARD_NAV: NavSection[] = [
  {
    items: [
      { title: "Overview", to: ROUTES.dashboard, icon: LayoutDashboard, exact: true },
      { title: "Bookings", to: ROUTES.dashboardBookings, icon: CalendarCheck },
      { title: "Availability", to: ROUTES.dashboardAvailability, icon: Clock },
    ],
  },
];
