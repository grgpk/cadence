import { createFileRoute } from "@tanstack/react-router";

import { GoogleCalendarCallback } from "../../../domains/shared/OAuthCallback";
import { ROUTES } from "../../../lib/routes";

export const Route = createFileRoute("/dashboard/calendar/callback")({
  validateSearch: (search) => ({ code: String(search.code ?? "") }),
  component: GoogleCalendarCallbackPage,
});

function GoogleCalendarCallbackPage() {
  return (
    <GoogleCalendarCallback
      code={Route.useSearch().code}
      returnTo={ROUTES.dashboardSettings}
    />
  );
}
