import type { LucideIcon } from "lucide-react";
import {
  Bug,
  CalendarCheck,
  KeyRound,
  LayoutDashboard,
  Users,
  Video,
} from "lucide-react";

export const ADMIN_SECTIONS = {
  MANAGEMENT: "Management",
  MONITOR: "Monitor",
} as const;

export type AdminSection = (typeof ADMIN_SECTIONS)[keyof typeof ADMIN_SECTIONS];
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
  {
    segment: "leads",
    title: "Leads",
    icon: Users,
    section: ADMIN_SECTIONS.MANAGEMENT,
  },
  {
    segment: "bookings",
    title: "Bookings",
    icon: CalendarCheck,
    section: ADMIN_SECTIONS.MANAGEMENT,
  },
  {
    segment: "calling-visits",
    title: "Calling Visits",
    icon: Video,
    section: ADMIN_SECTIONS.MANAGEMENT,
  },
  {
    segment: "roleaccesses",
    title: "Role Accesses",
    icon: KeyRound,
    section: ADMIN_SECTIONS.MANAGEMENT,
  },
  {
    segment: "bug-reports",
    title: "Bug Reports",
    icon: Bug,
    section: ADMIN_SECTIONS.MONITOR,
  },
];

export const ADMIN_OVERVIEW_ICON = LayoutDashboard;
