import { Link, useLocation } from "@tanstack/react-router";
import { CalendarCheck, LayoutDashboard, Settings2 } from "lucide-react";
import type { ReactNode } from "react";

import type { SessionUser } from "../../../bindings/SessionUser";
import {
  Sidebar,
  SidebarContent,
  SidebarFooter,
  SidebarGroup,
  SidebarGroupContent,
  SidebarGroupLabel,
  SidebarHeader,
  SidebarInset,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
  SidebarProvider,
  SidebarTrigger,
} from "../../../components/ui/sidebar";
import { UserDropdownSidenav } from "./user-dropdown-sidenav";

const DASHBOARD_SECTIONS = {
  MANAGEMENT: "Management",
} as const;

const DASHBOARD_NAV_ITEMS = [
  {
    title: "Bookings",
    to: "/dashboard/bookings",
    icon: CalendarCheck,
    section: DASHBOARD_SECTIONS.MANAGEMENT,
  },
  {
    title: "Availability",
    to: "/dashboard/availability",
    icon: Settings2,
    section: DASHBOARD_SECTIONS.MANAGEMENT,
  },
] as const;

const DASHBOARD_SECTION_ORDER = [DASHBOARD_SECTIONS.MANAGEMENT] as const;

function DashboardSidenav({ user }: { user: SessionUser }) {
  const { pathname } = useLocation();

  return (
    <Sidebar collapsible="icon">
      <SidebarHeader>
        <SidebarMenu>
          <SidebarMenuItem>
            <SidebarMenuButton
              size="lg"
              asChild
              isActive={pathname === "/dashboard"}
              tooltip="Dashboard Overview"
            >
              <Link to="/dashboard">
                <LayoutDashboard />
                <span className="font-medium">Dashboard Overview</span>
              </Link>
            </SidebarMenuButton>
          </SidebarMenuItem>
        </SidebarMenu>
      </SidebarHeader>
      <SidebarContent>
        {DASHBOARD_SECTION_ORDER.map((section) => (
          <SidebarGroup key={section}>
            <SidebarGroupLabel>{section}</SidebarGroupLabel>
            <SidebarGroupContent>
              <SidebarMenu>
                {DASHBOARD_NAV_ITEMS.filter((item) => item.section === section).map(
                  (item) => {
                    const Icon = item.icon;
                    return (
                      <SidebarMenuItem key={item.to}>
                        <SidebarMenuButton
                          asChild
                          isActive={pathname === item.to}
                          tooltip={item.title}
                        >
                          <Link to={item.to}>
                            <Icon />
                            <span>{item.title}</span>
                          </Link>
                        </SidebarMenuButton>
                      </SidebarMenuItem>
                    );
                  },
                )}
              </SidebarMenu>
            </SidebarGroupContent>
          </SidebarGroup>
        ))}
      </SidebarContent>
      <SidebarFooter className="border-t p-2">
        <UserDropdownSidenav user={user} />
      </SidebarFooter>
    </Sidebar>
  );
}

export function DashboardSidenavLayout({
  user,
  children,
}: {
  user: SessionUser;
  children: ReactNode;
}) {
  return (
    <SidebarProvider>
      <DashboardSidenav user={user} />
      <SidebarInset>
        <div className="flex items-center border-b p-2">
          <SidebarTrigger />
        </div>
        {children}
      </SidebarInset>
    </SidebarProvider>
  );
}
