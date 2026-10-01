import type { SessionUser } from "../bindings/SessionUser";
import { api } from "./api";

export const PUBLIC_PATHS = ["/", "/login", "/register"];

export function getSessionUser() {
  return api<SessionUser>("/api/auth/me").catch(() => null);
}

export function logout() {
  return api<void>("/api/auth/logout", { method: "POST" });
}
