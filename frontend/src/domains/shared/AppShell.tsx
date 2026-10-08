import { Link, useLocation, type LinkProps } from "@tanstack/react-router";
import type { LucideIcon } from "lucide-react";
import type { ReactNode } from "react";

import type { SessionUser } from "../../bindings/SessionUser";
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
} from "../../components/ui/sidebar";
import { APP_NAME } from "../../lib/config";
import { LogoMark } from "./Logo";
import { UserMenu } from "./UserMenu";

export type NavItem = {
  title: string;
  to: LinkProps["to"] & string;
  icon: LucideIcon;
  /** Match the path exactly, for an index page that prefixes its siblings. */
  exact?: boolean;
};

export type NavSection = { label?: string; items: NavItem[] };

function isActive(pathname: string, item: NavItem) {
  return item.exact
    ? pathname === item.to
    : pathname === item.to || pathname.startsWith(`${item.to}/`);
}

export function AppShell({
  user,
  home,
  area,
  sections,
  children,
}: {
  user: SessionUser;
  /** Where the logo links to. */
  home: NavItem["to"];
  /** Name of this part of the app, shown in the top bar. */
  area: string;
  sections: NavSection[];
  children: ReactNode;
}) {
  const { pathname } = useLocation();
  const current = sections
    .flatMap((section) => section.items)
    .find((item) => isActive(pathname, item));

  return (
    <SidebarProvider>
      <Sidebar collapsible="icon">
        <SidebarHeader>
          <SidebarMenu>
            <SidebarMenuItem>
              <SidebarMenuButton size="lg" asChild tooltip={APP_NAME}>
                <Link to={home}>
                  <span className="flex size-8 shrink-0 items-center justify-center">
                    <LogoMark />
                  </span>
                  <span className="font-display text-lg font-semibold tracking-tight">
                    {APP_NAME}
                  </span>
                </Link>
              </SidebarMenuButton>
            </SidebarMenuItem>
          </SidebarMenu>
        </SidebarHeader>
        <SidebarContent>
          {sections.map((section, index) => (
            <SidebarGroup key={section.label ?? index}>
              {section.label ? (
                <SidebarGroupLabel>{section.label}</SidebarGroupLabel>
              ) : null}
              <SidebarGroupContent>
                <SidebarMenu>
                  {section.items.map((item) => {
                    const Icon = item.icon;
                    return (
                      <SidebarMenuItem key={item.to}>
                        <SidebarMenuButton
                          asChild
                          isActive={isActive(pathname, item)}
                          tooltip={item.title}
                        >
                          <Link to={item.to}>
                            <Icon />
                            <span>{item.title}</span>
                          </Link>
                        </SidebarMenuButton>
                      </SidebarMenuItem>
                    );
                  })}
                </SidebarMenu>
              </SidebarGroupContent>
            </SidebarGroup>
          ))}
        </SidebarContent>
        <SidebarFooter className="border-t p-2">
          <UserMenu user={user} />
        </SidebarFooter>
      </Sidebar>
      <SidebarInset>
        <div className="flex h-12 items-center gap-2 border-b bg-card px-3 text-sm">
          <SidebarTrigger />
          <span className="text-muted-foreground">{area}</span>
          {current ? (
            <>
              <span aria-hidden="true" className="text-muted-foreground">
                /
              </span>
              <span className="font-medium">{current.title}</span>
            </>
          ) : null}
        </div>
        <div className="mx-auto flex w-full max-w-5xl flex-1 flex-col gap-6 p-4 md:p-8">
          {children}
        </div>
      </SidebarInset>
    </SidebarProvider>
  );
}
