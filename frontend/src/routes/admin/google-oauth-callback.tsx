import { createFileRoute } from "@tanstack/react-router";

import { GoogleCalendarCallback } from "../../domains/shared/OAuthCallback";
import { ROUTES } from "../../lib/routes";

export const Route = createFileRoute("/admin/google-oauth-callback")({
  validateSearch: (search) => ({ code: String(search.code ?? "") }),
  component: GoogleOAuthCallbackPage,
});

function GoogleOAuthCallbackPage() {
  return <GoogleCalendarCallback code={Route.useSearch().code} returnTo={ROUTES.admin} />;
}
