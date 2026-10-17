import { Link, useLocation, type LinkProps } from "@tanstack/react-router";
import { cn } from "cn";
import { ChevronRight, type LucideIcon } from "lucide-react";
import { type ReactNode, useEffect, useState } from "react";

import type { SessionUser } from "../../bindings/SessionUser";
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from "../../components/ui/collapsible";
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
  SidebarMenuSub,
  SidebarMenuSubButton,
  SidebarMenuSubItem,
  SidebarProvider,
  SidebarTrigger,
  useSidebar,
} from "../../components/ui/sidebar";
import { hasAdminAccess } from "../../lib/auth";
import { APP_NAME } from "../../lib/config";
import { ROUTES } from "../../lib/routes";
import { LogoMark } from "./logo";
import { SEGMENT, SEGMENT_ON, SEGMENT_TRACK } from "./segmented-control";
import { ThemeToggle } from "./theme-toggle";
import { UserMenu } from "./user-menu";

type NavLink = {
  title: string;
  to: LinkProps["to"] & string;
  /** Match the path exactly, for an index page that prefixes its siblings. */
  exact?: boolean;
};

export type NavItem = NavLink & {
  icon: LucideIcon;
  /** Pages under this one, shown as a group that opens and closes. */
  items?: NavLink[];
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

function isActive(pathname: string, item: NavLink) {
  return item.exact
    ? pathname === item.to
    : pathname === item.to || pathname.startsWith(`${item.to}/`);
}

/** The page a path is on: the most specific link that matches it. */
function activeLink<T extends NavLink>(pathname: string, links: T[]): T | undefined {
  return links
    .filter((link) => isActive(pathname, link))
    .sort((a, b) => b.to.length - a.to.length)[0];
}

/** A nav item with pages under it. Collapsed to icons, it opens its first page. */
/** The page you are on lifts off the sidebar like a card, its icon in dusk. */
const ACTIVE_NAV =
  "data-[active=true]:bg-card data-[active=true]:shadow-card data-[active=true]:[&>svg]:text-dusk";

function NavGroup({
  item,
  items,
  pathname,
}: {
  item: NavItem;
  items: NavLink[];
  pathname: string;
}) {
  const { state, isMobile } = useSidebar();
  const active = isActive(pathname, item);
  const current = activeLink(pathname, items);
  const [open, setOpen] = useState(active);
  // Arriving at one of its pages from elsewhere opens the group.
  useEffect(() => {
    if (active) setOpen(true);
  }, [active]);
  const Icon = item.icon;

  if (state === "collapsed" && !isMobile) {
    return (
      <SidebarMenuItem>
        <SidebarMenuButton
          asChild
          isActive={active}
          tooltip={item.title}
          className={ACTIVE_NAV}
        >
          <Link to={items[0]?.to ?? item.to}>
            <Icon />
            <span>{item.title}</span>
          </Link>
        </SidebarMenuButton>
      </SidebarMenuItem>
    );
  }

  return (
    <Collapsible asChild open={open} onOpenChange={setOpen} className="group/collapsible">
      <SidebarMenuItem>
        <CollapsibleTrigger asChild>
          <SidebarMenuButton tooltip={item.title}>
            <Icon />
            <span>{item.title}</span>
            <ChevronRight className="ml-auto transition-transform duration-200 group-data-[state=open]/collapsible:rotate-90" />
          </SidebarMenuButton>
        </CollapsibleTrigger>
        <CollapsibleContent className="overflow-hidden data-[state=closed]:animate-collapsible-up data-[state=open]:animate-collapsible-down">
          <SidebarMenuSub>
            {items.map((sub) => (
              <SidebarMenuSubItem key={sub.to}>
                <SidebarMenuSubButton asChild isActive={sub === current}>
                  <Link to={sub.to}>
                    <span>{sub.title}</span>
                  </Link>
                </SidebarMenuSubButton>
              </SidebarMenuSubItem>
            ))}
          </SidebarMenuSub>
        </CollapsibleContent>
      </SidebarMenuItem>
    </Collapsible>
  );
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
  const current = activeLink(
    pathname,
    sections.flatMap((section) =>
      section.items.flatMap((item) => [...(item.items ?? []), item]),
    ),
  );

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
                    if (item.items) {
                      return (
                        <NavGroup
                          key={item.to}
                          item={item}
                          items={item.items}
                          pathname={pathname}
                        />
                      );
                    }
                    const Icon = item.icon;
                    return (
                      <SidebarMenuItem key={item.to}>
                        <SidebarMenuButton
                          asChild
                          isActive={isActive(pathname, item)}
                          tooltip={item.title}
                          className={ACTIVE_NAV}
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
      <SidebarInset className="bg-canvas">
        <header className="sticky top-0 z-20 flex h-12 items-center gap-2 border-b bg-canvas/85 px-3 text-sm backdrop-blur">
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
        </header>
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
