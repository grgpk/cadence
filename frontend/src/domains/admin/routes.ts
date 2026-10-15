import {
  Bug,
  CalendarCheck,
  KeyRound,
  LayoutDashboard,
  Mail,
  UserCog,
  Users,
  Video,
} from "lucide-react";

import { queryOptions } from "@tanstack/react-query";

import { api } from "../../lib/api";
import { QUERY_KEYS } from "../../lib/query-keys";
import { ROUTES } from "../../lib/routes";
import type { NavItem, NavSection } from "../shared/app-shell";

const ADMIN_ROUTES = {
  leads: "/admin/leads",
  bookings: "/admin/bookings",
  callingVisits: "/admin/calling-visits",
  hosts: "/admin/hosts",
  roleAccesses: "/admin/roleaccesses",
  emails: "/admin/emails",
  bugReports: "/admin/bug-reports",
} as const;

type AdminPage = NavItem & {
  description: string;
  /** Where the page's rows come from. Also its cache key, shared with the overview. */
  endpoint: string;
};

export const VISIT_FUNNEL_ENDPOINT = "/api/admin/calling-visits/funnel";

/** The admin list pages: the sidebar, each page and the overview read from this. */
export const ADMIN_PAGES = {
  leads: {
    title: "Leads",
    description: "People who filled in the booking form.",
    to: ADMIN_ROUTES.leads,
    endpoint: "/api/admin/leads",
    icon: Users,
  },
  bookings: {
    title: "Bookings",
    description: "Booked calls across all hosts.",
    to: ADMIN_ROUTES.bookings,
    endpoint: "/api/admin/bookings",
    icon: CalendarCheck,
  },
  callingVisits: {
    title: "Calling visits",
    description: "Visits to the booking page.",
    to: ADMIN_ROUTES.callingVisits,
    endpoint: "/api/admin/calling-visits",
    icon: Video,
  },
  hosts: {
    title: "Hosts",
    description: "People with a booking page and how far each one is set up.",
    to: ADMIN_ROUTES.hosts,
    endpoint: "/api/admin/hosts",
    icon: UserCog,
  },
  roleAccesses: {
    title: "Role accesses",
    description: "Roles granted to users.",
    to: ADMIN_ROUTES.roleAccesses,
    endpoint: "/api/admin/roleaccesses",
    icon: KeyRound,
  },
  emails: {
    title: "Emails",
    description: "Confirmations and reminders, sent and waiting.",
    to: ADMIN_ROUTES.emails,
    endpoint: "/api/admin/emails",
    icon: Mail,
  },
  bugReports: {
    title: "Bug reports",
    description: "Errors captured by the application.",
    to: ADMIN_ROUTES.bugReports,
    endpoint: "/api/admin/bug-reports",
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
      ADMIN_PAGES.hosts,
      ADMIN_PAGES.roleAccesses,
    ],
  },
  { label: "Monitor", items: [ADMIN_PAGES.emails, ADMIN_PAGES.bugReports] },
];

/** What an admin endpoint returns, cached under the endpoint so pages share it. */
export function adminDataQuery<T>(endpoint: string) {
  return queryOptions({
    queryKey: QUERY_KEYS.adminData(endpoint),
    queryFn: () => api<T>(endpoint),
  });
}
