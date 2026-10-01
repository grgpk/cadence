import { Link, useLocation } from "@tanstack/react-router";
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
import {
  ADMIN_NAV_ITEMS,
  ADMIN_OVERVIEW_ICON as LayoutDashboard,
  adminRoutePath,
  type AdminSection,
} from "../routes";
import { UserDropdownSidenav } from "./user-dropdown-sidenav";

const SECTIONS: AdminSection[] = ["Management", "Monitor"];

function AdminSidenav({ user }: { user: SessionUser }) {
  const { pathname } = useLocation();

  return (
    <Sidebar collapsible="icon">
      <SidebarHeader>
        <SidebarMenu>
          <SidebarMenuItem>
            <SidebarMenuButton
              size="lg"
              asChild
              isActive={pathname === "/dashboard/admin"}
              tooltip="Admin Overview"
            >
              <Link to="/dashboard/admin">
                <LayoutDashboard />
                <span className="font-medium">Admin Overview</span>
              </Link>
            </SidebarMenuButton>
          </SidebarMenuItem>
        </SidebarMenu>
      </SidebarHeader>
      <SidebarContent>
        {SECTIONS.map((section) => (
          <SidebarGroup key={section}>
            <SidebarGroupLabel>{section}</SidebarGroupLabel>
            <SidebarGroupContent>
              <SidebarMenu>
                {ADMIN_NAV_ITEMS.filter((item) => item.section === section).map(
                  (item) => {
                    const path = adminRoutePath(item.segment);
                    const Icon = item.icon;
                    return (
                      <SidebarMenuItem key={item.segment}>
                        <SidebarMenuButton
                          asChild
                          isActive={pathname === path}
                          tooltip={item.title}
                        >
                          <Link to={path}>
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

export function AdminSidenavLayout({
  user,
  children,
}: {
  user: SessionUser;
  children: ReactNode;
}) {
  return (
    <SidebarProvider>
      <AdminSidenav user={user} />
      <SidebarInset>
        <div className="flex items-center border-b p-2">
          <SidebarTrigger />
        </div>
        {children}
      </SidebarInset>
    </SidebarProvider>
  );
}
