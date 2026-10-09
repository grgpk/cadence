import { useMutation } from "@tanstack/react-query";
import { createFileRoute, Link } from "@tanstack/react-router";
import { Loader2 } from "lucide-react";
import { useState } from "react";

import type { ForgotPasswordRequest } from "../bindings/ForgotPasswordRequest";
import { Button } from "../components/ui/button";
import { Input } from "../components/ui/input";
import { Label } from "../components/ui/label";
import { AuthCard } from "../domains/shared/AuthCard";
import { RouteError } from "../domains/shared/NotFound";
import { api } from "../lib/api";
import { APP_NAME, NOINDEX_META } from "../lib/config";
import { ROUTES } from "../lib/routes";

export const Route = createFileRoute("/forgot-password")({
  head: () => ({ meta: [{ title: `Reset your password | ${APP_NAME}` }, NOINDEX_META] }),
  component: ForgotPasswordPage,
  errorComponent: RouteError,
});

function ForgotPasswordPage() {
  // The address the link was sent to, once it has been asked for.
  const [sentTo, setSentTo] = useState<string | null>(null);
  const request = useMutation({
    mutationFn: (request: ForgotPasswordRequest) =>
      api<void>("/api/auth/forgot-password", {
        method: "POST",
        body: JSON.stringify(request),
      }),
    onSuccess: (_, { email }) => setSentTo(email),
  });

  return (
    <AuthCard
      title={sentTo ? "Check your email" : "Reset your password"}
      description={
        sentTo
          ? `If ${sentTo} has an account, a link to choose a new password is on its way. It works for one hour.`
          : "Enter the email you signed up with and we will send you a link."
      }
      error={request.error}
      onSubmit={(event) => {
        event.preventDefault();
        const email = String(new FormData(event.currentTarget).get("email") ?? "").trim();
        request.mutate({ email });
      }}
      footer={
        <Link
          to={ROUTES.login}
          className="font-medium text-foreground underline underline-offset-4"
        >
          Back to log in
        </Link>
      }
    >
      {sentTo ? (
        <Button type="button" variant="outline" onClick={() => setSentTo(null)}>
          Use a different email
        </Button>
      ) : (
        <>
          <div className="grid gap-2">
            <Label htmlFor="email">Email</Label>
            <Input id="email" name="email" type="email" autoComplete="email" required />
          </div>
          <Button className="mt-2 w-full" disabled={request.isPending}>
            {request.isPending ? <Loader2 className="animate-spin" /> : null}
            {request.isPending ? "Sending link" : "Send reset link"}
          </Button>
        </>
      )}
    </AuthCard>
  );
}
