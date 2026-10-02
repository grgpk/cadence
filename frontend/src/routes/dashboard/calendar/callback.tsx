import { useMutation } from "@tanstack/react-query";
import { useEffect, useRef } from "react";
import { createFileRoute, useNavigate } from "@tanstack/react-router";

import { Card, CardContent, CardHeader, CardTitle } from "../../../components/ui/card";
import { submitGoogleCalendarCode } from "../../../domains/widget/cadence-api";

export const Route = createFileRoute("/dashboard/calendar/callback")({
  validateSearch: (search) => ({ code: String(search.code ?? "") }),
  component: GoogleCalendarCallbackPage,
});

function GoogleCalendarCallbackPage() {
  const navigate = useNavigate();
  const { code } = Route.useSearch();
  const submitted = useRef(false);
  const connectMutation = useMutation({
    mutationFn: submitGoogleCalendarCode,
    onSuccess: () => navigate({ to: "/dashboard" }),
  });

  useEffect(() => {
    if (!code || submitted.current) return;
    submitted.current = true;
    connectMutation.mutate(code);
  }, [code, connectMutation]);

  const error = connectMutation.error;

  return (
    <section className="flex flex-1 items-center justify-center">
      <Card className="w-full max-w-md">
        <CardHeader>
          <CardTitle>
            {error ? "Connection failed" : "Connecting Google Calendar"}
          </CardTitle>
        </CardHeader>
        <CardContent>
          <p className="text-sm text-muted-foreground">
            {error instanceof Error
              ? error.message
              : error
                ? "Google Calendar connection failed."
                : "Saving your Google Calendar connection..."}
          </p>
        </CardContent>
      </Card>
    </section>
  );
}
