import type { Role } from "../bindings/Role";
import type { SessionUser } from "../bindings/SessionUser";
import { api } from "./api";

export const PUBLIC_PATHS = ["/", "/login", "/register"];
const ROLES = {
  ROOT: "Root",
  ADMIN: "Admin",
  HOST: "Host",
} as const satisfies Record<Uppercase<Role>, Role>;

const ADMIN_ROLES: ReadonlySet<Role> = new Set([ROLES.ROOT, ROLES.ADMIN]);

export function hasAdminAccess(user: Pick<SessionUser, "roles"> | null | undefined) {
  return user?.roles.some((role) => ADMIN_ROLES.has(role)) ?? false;
}

export function getSessionUser() {
  return api<SessionUser>("/api/auth/me").catch(() => null);
}

export function logout() {
  return api<void>("/api/auth/logout", { method: "POST" });
}
