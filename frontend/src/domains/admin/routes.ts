import {
  Bug,
  CalendarCheck,
  KeyRound,
  LayoutDashboard,
  Users,
  Video,
} from "lucide-react";

import { ROUTES } from "../../lib/routes";
import type { NavItem, NavSection } from "../shared/AppShell";

const ADMIN_ROUTES = {
  leads: "/admin/leads",
  bookings: "/admin/bookings",
  callingVisits: "/admin/calling-visits",
  roleAccesses: "/admin/roleaccesses",
  bugReports: "/admin/bug-reports",
} as const;

type AdminPage = NavItem & { description: string };

/** The admin list pages: the sidebar and the overview both render from this. */
export const ADMIN_PAGES = {
  leads: {
    title: "Leads",
    description: "People who filled in the booking form.",
    to: ADMIN_ROUTES.leads,
    icon: Users,
  },
  bookings: {
    title: "Bookings",
    description: "Booked calls across all hosts.",
    to: ADMIN_ROUTES.bookings,
    icon: CalendarCheck,
  },
  callingVisits: {
    title: "Calling visits",
    description: "Visits to the booking page.",
    to: ADMIN_ROUTES.callingVisits,
    icon: Video,
  },
  roleAccesses: {
    title: "Role accesses",
    description: "Roles granted to users.",
    to: ADMIN_ROUTES.roleAccesses,
    icon: KeyRound,
  },
  bugReports: {
    title: "Bug reports",
    description: "Errors captured by the application.",
    to: ADMIN_ROUTES.bugReports,
    icon: Bug,
  },
} as const satisfies Record<keyof typeof ADMIN_ROUTES, AdminPage>;

export const ADMIN_NAV: NavSection[] = [
  {
    items: [{ title: "Overview", to: ROUTES.admin, icon: LayoutDashboard, exact: true }],
  },
  {
    label: "Management",
    items: [
      ADMIN_PAGES.leads,
      ADMIN_PAGES.bookings,
      ADMIN_PAGES.callingVisits,
      ADMIN_PAGES.roleAccesses,
    ],
  },
  { label: "Monitor", items: [ADMIN_PAGES.bugReports] },
];
