import { Link, useLocation, type LinkProps } from "@tanstack/react-router";
import { cn } from "cn";
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
import { hasAdminAccess } from "../../lib/auth";
import { APP_NAME } from "../../lib/config";
import { ROUTES } from "../../lib/routes";
import { LogoMark } from "./logo";
import { SEGMENT, SEGMENT_ON, SEGMENT_TRACK } from "./segmented-control";
import { ThemeToggle } from "./theme-toggle";
import { UserMenu } from "./user-menu";

export type NavItem = {
  title: string;
  to: LinkProps["to"] & string;
  icon: LucideIcon;
  /** Match the path exactly, for an index page that prefixes its siblings. */
  exact?: boolean;
};

export type NavSection = { label?: string; items: NavItem[] };

/** The two parts of the app. Admins move between them from the top bar. */
const AREAS = [
  { title: "Dashboard", to: ROUTES.dashboard },
  { title: "Admin", to: ROUTES.admin },
] as const;

type Area = (typeof AREAS)[number]["title"];

const CONTENT_ID = "content";

function AreaSwitch({ current }: { current: Area }) {
  return (
    <div className={SEGMENT_TRACK}>
      {AREAS.map((area) => (
        <Link
          key={area.to}
          to={area.to}
          aria-current={area.title === current ? "true" : undefined}
          className={cn(SEGMENT, "px-2.5 py-1", area.title === current && SEGMENT_ON)}
        >
          {area.title}
        </Link>
      ))}
    </div>
  );
}

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
  wide = false,
  children,
}: {
  user: SessionUser;
  /** Where the logo links to. */
  home: NavItem["to"];
  /** The part of the app this shell wraps, shown in the top bar. */
  area: Area;
  sections: NavSection[];
  /** Room for wide tables. */
  wide?: boolean;
  children: ReactNode;
}) {
  const { pathname } = useLocation();
  const current = sections
    .flatMap((section) => section.items)
    .find((item) => isActive(pathname, item));

  return (
    <SidebarProvider>
      <a
        href={`#${CONTENT_ID}`}
        className="sr-only z-50 rounded-md bg-primary px-3 py-2 text-sm font-medium text-primary-foreground focus:not-sr-only focus:fixed focus:top-2 focus:left-2"
      >
        Skip to content
      </a>
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
          <nav aria-label="Where you are" className="flex min-w-0 items-center gap-2">
            {hasAdminAccess(user) ? (
              <AreaSwitch current={area} />
            ) : (
              <span className="text-muted-foreground">{area}</span>
            )}
            {current ? (
              <>
                <span aria-hidden="true" className="text-muted-foreground">
                  /
                </span>
                <span aria-current="page" className="truncate font-medium">
                  {current.title}
                </span>
              </>
            ) : null}
          </nav>
          <div className="ml-auto">
            <ThemeToggle />
          </div>
        </div>
        <div
          id={CONTENT_ID}
          tabIndex={-1}
          className={cn(
            "mx-auto flex w-full flex-1 flex-col gap-6 p-4 outline-none md:p-8",
            wide ? "max-w-7xl" : "max-w-5xl",
          )}
        >
          {children}
        </div>
      </SidebarInset>
    </SidebarProvider>
  );
}
