import { createServerFn } from "@tanstack/react-start";
import { getCookie } from "@tanstack/react-start/server";
import type { Role } from "../bindings/Role";
import type { SessionUser } from "../bindings/SessionUser";
import { SERVER_API_URL } from "./api-url";
import { api } from "./api";
import { isBookingPath, ROUTES } from "./routes";

const PUBLIC_PATHS: readonly string[] = [
  ROUTES.home,
  ROUTES.bookingConfirmation,
  ROUTES.login,
  ROUTES.privacy,
  ROUTES.register,
  ROUTES.terms,
];

/** Pages a visitor can open without an account. */
export function isPublicPath(pathname: string): boolean {
  return PUBLIC_PATHS.includes(pathname) || isBookingPath(pathname);
}

const ROLES = {
  ROOT: "Root",
  ADMIN: "Admin",
  HOST: "Host",
} as const satisfies Record<Uppercase<Role>, Role>;

const ADMIN_ROLES: ReadonlySet<Role> = new Set([ROLES.ROOT, ROLES.ADMIN]);

export function hasAdminAccess(user: Pick<SessionUser, "roles"> | null | undefined) {
  return user?.roles.some((role) => ADMIN_ROLES.has(role)) ?? false;
}

export const getSessionUser = createServerFn({ method: "GET" }).handler(async () => {
  const session = getCookie("session");
  if (!session) return null;

  const response = await fetch(`${SERVER_API_URL}/api/auth/me`, {
    headers: { Cookie: `session=${session}` },
  });
  if (!response.ok) return null;

  return response.json() as Promise<SessionUser>;
});

export function logout() {
  return api<void>("/api/auth/logout", { method: "POST" });
}
