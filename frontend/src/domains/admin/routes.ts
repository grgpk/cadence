import type { LucideIcon } from "lucide-react";
import {
  Bug,
  CalendarCheck,
  KeyRound,
  LayoutDashboard,
  Users,
  Video,
} from "lucide-react";

export type AdminSection = "Management" | "Monitor";
type AdminRouteSegment =
  | "leads"
  | "bookings"
  | "calling-visits"
  | "roleaccesses"
  | "bug-reports";

const ADMIN_ROUTE_PATHS = {
  leads: "/dashboard/admin/leads",
  bookings: "/dashboard/admin/bookings",
  "calling-visits": "/dashboard/admin/calling-visits",
  roleaccesses: "/dashboard/admin/roleaccesses",
  "bug-reports": "/dashboard/admin/bug-reports",
} as const;

type AdminNavItem = {
  segment: AdminRouteSegment;
  title: string;
  icon: LucideIcon;
  section: AdminSection;
};

export function adminRoutePath(segment: AdminRouteSegment) {
  return ADMIN_ROUTE_PATHS[segment];
}

export const ADMIN_NAV_ITEMS: AdminNavItem[] = [
  { segment: "leads", title: "Leads", icon: Users, section: "Management" },
  { segment: "bookings", title: "Bookings", icon: CalendarCheck, section: "Management" },
  {
    segment: "calling-visits",
    title: "Calling Visits",
    icon: Video,
    section: "Management",
  },
  {
    segment: "roleaccesses",
    title: "Role Accesses",
    icon: KeyRound,
    section: "Management",
  },
  { segment: "bug-reports", title: "Bug Reports", icon: Bug, section: "Monitor" },
];

export const ADMIN_OVERVIEW_ICON = LayoutDashboard;
