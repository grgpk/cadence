import { useMutation } from "@tanstack/react-query";
import { useEffect, useRef } from "react";
import { createFileRoute, useNavigate } from "@tanstack/react-router";

import { submitGoogleCalendarCode } from "../../domains/widget/cadence-api";

export const Route = createFileRoute("/admin/google-oauth-callback")({
  validateSearch: (search) => ({ code: String(search.code ?? "") }),
  component: GoogleOAuthCallbackPage,
});

function GoogleOAuthCallbackPage() {
  const navigate = useNavigate();
  const { code } = Route.useSearch();
  const submitted = useRef(false);
  const connectMutation = useMutation({
    mutationFn: submitGoogleCalendarCode,
    onSuccess: () => navigate({ to: "/admin" }),
  });

  useEffect(() => {
    if (!code || submitted.current) return;
    submitted.current = true;
    connectMutation.mutate(code);
  }, [code, connectMutation]);

  const error = connectMutation.error;

  return (
    <section className="flex flex-1 items-center justify-center p-6">
      <div className="text-center">
        {error ? (
          <p className="rounded-md bg-red-100 p-4 text-red-700">
            {error instanceof Error ? error.message : "Google OAuth connection failed."}
          </p>
        ) : (
          <p className="text-sm text-muted-foreground">Connecting Google Calendar...</p>
        )}
      </div>
    </section>
  );
}
