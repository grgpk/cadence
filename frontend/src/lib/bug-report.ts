import type { AddBugReportRequest } from "../bindings/AddBugReportRequest";
import type { AdminBugType } from "../bindings/AdminBugType";
import { ApiError } from "./api";
import { API_URL } from "./config";

/** The endpoint is open to anyone, so one page load only ever sends a few. */
const MAX_REPORTS = 5;
const reported = new Set<string>();

/** Tells the admin bug list about an error in the browser. Each message is sent once. */
export function reportError(type: AdminBugType, error: unknown): void {
  // A failed request is already known to the server that answered it.
  if (error instanceof ApiError) return;
  const message = error instanceof Error ? error.message : String(error);
  if (reported.has(message) || reported.size >= MAX_REPORTS) return;
  reported.add(message);

  const report: AddBugReportRequest = {
    bug_type: type,
    message,
    user_login: null,
    stack_trace: error instanceof Error ? (error.stack ?? null) : null,
  };
  // Plain fetch: a failure here must not come back around as another error to report.
  void fetch(`${API_URL}/api/bug-reports`, {
    method: "POST",
    keepalive: true,
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(report),
  }).catch(() => undefined);
}

/** Reports errors nothing caught. Returns the function that stops watching. */
export function watchErrors(): () => void {
  const onError = (event: ErrorEvent) =>
    reportError("JsError", event.error ?? event.message);
  const onRejection = (event: PromiseRejectionEvent) =>
    reportError("PromiseRejection", event.reason);
  window.addEventListener("error", onError);
  window.addEventListener("unhandledrejection", onRejection);
  return () => {
    window.removeEventListener("error", onError);
    window.removeEventListener("unhandledrejection", onRejection);
  };
}
