import { createServerFn } from "@tanstack/react-start";
import { getCookie } from "@tanstack/react-start/server";
import type { Role } from "../bindings/Role";
import type { SessionUser } from "../bindings/SessionUser";
import { SERVER_API_URL } from "./api-url";
import { api, UNAUTHORIZED } from "./api";

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
  if (response.status === UNAUTHORIZED) return null;
  // An outage is not a logout: let the error page show instead of the login form.
  if (!response.ok) throw new Error(`Session check failed: ${response.status}`);

  return response.json() as Promise<SessionUser>;
});

export function logout() {
  return api<void>("/api/auth/logout", { method: "POST" });
}
