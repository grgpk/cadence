import { useMutation } from "@tanstack/react-query";
import { createFileRoute, Link } from "@tanstack/react-router";
import { Loader2 } from "lucide-react";
import { useState } from "react";

import type { ResetPasswordRequest } from "../bindings/ResetPasswordRequest";
import { Button } from "../components/ui/button";
import { Label } from "../components/ui/label";
import { AuthCard, PasswordInput } from "../domains/shared/AuthCard";
import { RouteError } from "../domains/shared/NotFound";
import { api } from "../lib/api";
import { APP_NAME, NOINDEX_META } from "../lib/config";
import { ROUTES } from "../lib/routes";
import { searchText } from "../lib/search";

const RESET_ERRORS = {
  400: "This reset link is no longer valid. Links work once, for one hour. Ask for a new one.",
};

export const Route = createFileRoute("/reset-password")({
  validateSearch: (search) => ({ token: searchText(search.token, /^[\w-]{16,256}$/) }),
  head: () => ({
    meta: [{ title: `Choose a new password | ${APP_NAME}` }, NOINDEX_META],
  }),
  component: ResetPasswordPage,
  errorComponent: RouteError,
});

function ResetPasswordPage() {
  const { token } = Route.useSearch();
  const [done, setDone] = useState(false);
  const reset = useMutation({
    mutationFn: (request: ResetPasswordRequest) =>
      api<void>("/api/auth/reset-password", {
        method: "POST",
        body: JSON.stringify(request),
      }),
    onSuccess: () => setDone(true),
  });
  const newLink = (
    <Link
      to={ROUTES.forgotPassword}
      className="font-medium text-foreground underline underline-offset-4"
    >
      Ask for a new link
    </Link>
  );

  if (!token) {
    return (
      <AuthCard
        title="This link is incomplete"
        description="Open the link from your email again, or ask for a new one."
        error={null}
        onSubmit={(event) => event.preventDefault()}
        footer={newLink}
      >
        {null}
      </AuthCard>
    );
  }

  return (
    <AuthCard
      title={done ? "Password changed" : "Choose a new password"}
      description={
        done
          ? "Log in with your new password."
          : "It replaces your old password as soon as you save it."
      }
      error={reset.error}
      errorMessages={RESET_ERRORS}
      onSubmit={(event) => {
        event.preventDefault();
        reset.mutate({
          token,
          password: String(new FormData(event.currentTarget).get("password") ?? ""),
        });
      }}
      footer={done ? null : newLink}
    >
      {done ? (
        <Button asChild>
          <Link to={ROUTES.login}>Log in</Link>
        </Button>
      ) : (
        <>
          <div className="grid gap-2">
            <Label htmlFor="password">New password</Label>
            <PasswordInput
              id="password"
              name="password"
              autoComplete="new-password"
              minLength={8}
              aria-describedby="password-hint"
              required
            />
            <p id="password-hint" className="text-xs text-muted-foreground">
              At least 8 characters.
            </p>
          </div>
          <Button className="mt-2 w-full" disabled={reset.isPending}>
            {reset.isPending ? <Loader2 className="animate-spin" /> : null}
            {reset.isPending ? "Saving" : "Save new password"}
          </Button>
        </>
      )}
    </AuthCard>
  );
}
