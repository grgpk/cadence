import { Link, useNavigate } from "@tanstack/react-router";
import { ChevronsUpDownIcon, LogOutIcon, MoonIcon, SunIcon } from "lucide-react";

import type { SessionUser } from "../../bindings/SessionUser";
import { Avatar, AvatarFallback } from "../../components/ui/avatar";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuGroup,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "../../components/ui/dropdown-menu";
import { SidebarMenuButton, useSidebar } from "../../components/ui/sidebar";
import { hasAdminAccess, logout } from "../../lib/auth";
import { ROUTES } from "../../lib/routes";
import { THEMES, useTheme } from "../../lib/theme";

export function UserMenu({ user }: { user: SessionUser }) {
  const navigate = useNavigate();
  const { isMobile } = useSidebar();
  const userName = user.full_name || "User";
  const userInitial = userName.charAt(0).toUpperCase();
  const canAccessAdmin = hasAdminAccess(user);
  const { theme, toggleTheme } = useTheme();
  const isDark = theme === THEMES.DARK;

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <SidebarMenuButton
          size="lg"
          className="data-[state=open]:bg-sidebar-accent data-[state=open]:text-sidebar-accent-foreground"
        >
          <Avatar className="size-8 rounded-lg">
            <AvatarFallback className="rounded-lg">{userInitial}</AvatarFallback>
          </Avatar>
          <div className="grid flex-1 text-left text-sm leading-tight">
            <span className="truncate font-medium">{userName}</span>
            <span className="truncate text-xs text-muted-foreground">{user.email}</span>
          </div>
          <ChevronsUpDownIcon className="ml-auto size-4" />
        </SidebarMenuButton>
      </DropdownMenuTrigger>
      <DropdownMenuContent
        className="min-w-60 rounded-lg"
        side={isMobile ? "bottom" : "right"}
        align="end"
        sideOffset={4}
      >
        <DropdownMenuLabel className="p-0 font-normal">
          <div className="flex items-center gap-2 px-1 py-1.5 text-left text-sm">
            <Avatar className="size-8 rounded-lg">
              <AvatarFallback className="rounded-lg">{userInitial}</AvatarFallback>
            </Avatar>
            <div className="grid flex-1 leading-tight">
              <span className="truncate font-medium">{userName}</span>
              <span className="truncate text-xs text-muted-foreground">{user.email}</span>
            </div>
          </div>
        </DropdownMenuLabel>
        <DropdownMenuSeparator />
        <DropdownMenuGroup>
          <DropdownMenuItem asChild>
            <Link to={ROUTES.dashboard}>Dashboard</Link>
          </DropdownMenuItem>
          {canAccessAdmin ? (
            <DropdownMenuItem asChild>
              <Link to={ROUTES.admin}>Admin</Link>
            </DropdownMenuItem>
          ) : null}
        </DropdownMenuGroup>
        <DropdownMenuSeparator />
        <DropdownMenuItem
          onSelect={(event) => {
            event.preventDefault();
            toggleTheme();
          }}
        >
          {isDark ? <SunIcon /> : <MoonIcon />}
          {isDark ? "Light theme" : "Dark theme"}
        </DropdownMenuItem>
        <DropdownMenuItem
          onClick={() => {
            void logout().then(() => navigate({ to: ROUTES.login }));
          }}
        >
          <LogOutIcon />
          Log out
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
