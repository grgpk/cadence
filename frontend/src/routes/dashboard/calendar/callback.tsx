import { createFileRoute } from "@tanstack/react-router";

import { GoogleCalendarCallback } from "../../../domains/shared/oauth-callback";
import { ROUTES } from "../../../lib/routes";

export const Route = createFileRoute("/dashboard/calendar/callback")({
  validateSearch: (search) => ({
    code: String(search.code ?? ""),
    state: String(search.state ?? ""),
  }),
  component: GoogleCalendarCallbackPage,
});

function GoogleCalendarCallbackPage() {
  return (
    <GoogleCalendarCallback
      code={Route.useSearch().code}
      state={Route.useSearch().state}
      returnTo={ROUTES.dashboardAppsInstalled}
    />
  );
}
