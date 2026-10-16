import { useMutation, useQueryClient } from "@tanstack/react-query";
import { createFileRoute, Link, redirect, useNavigate } from "@tanstack/react-router";
import { Loader2 } from "lucide-react";

import type { RegisterRequest } from "../bindings/RegisterRequest";
import { Button } from "../components/ui/button";
import { Input } from "../components/ui/input";
import { Label } from "../components/ui/label";
import { AuthCard, PasswordInput } from "../domains/shared/auth-card";
import { ConsentNotice } from "../domains/shared/consent-notice";
import { RouteError } from "../domains/shared/not-found";
import { api } from "../lib/api";
import { getSessionUser } from "../lib/auth";
import { APP_NAME, NOINDEX_META } from "../lib/config";
import { ROUTES } from "../lib/routes";

const REGISTER_ERRORS = {
  400: "Check your name and email, and use a password of at least 8 characters.",
  409: "An account with this email already exists.",
};

export const Route = createFileRoute("/register")({
  beforeLoad: async () => {
    if (await getSessionUser()) throw redirect({ to: ROUTES.dashboard });
  },
  head: () => ({ meta: [{ title: `Create account | ${APP_NAME}` }, NOINDEX_META] }),
  component: RegisterPage,
  errorComponent: RouteError,
});

function RegisterPage() {
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const registerMutation = useMutation({
    mutationFn: (request: RegisterRequest) =>
      api<void>("/api/auth/register", {
        method: "POST",
        body: JSON.stringify(request),
      }),
    onSuccess: () => {
      // Anything cached belongs to whoever used this tab before.
      queryClient.clear();
      void navigate({ to: ROUTES.dashboard });
    },
  });

  function submit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const form = new FormData(event.currentTarget);
    registerMutation.mutate({
      full_name: String(form.get("full_name") ?? ""),
      email: String(form.get("email") ?? ""),
      password: String(form.get("password") ?? ""),
    });
  }

  return (
    <AuthCard
      title="Create your account"
      description="Set your hours, share one link, and let people book you."
      error={registerMutation.error}
      errorMessages={REGISTER_ERRORS}
      onSubmit={submit}
      footer={
        <>
          Already have an account?{" "}
          <Link
            to={ROUTES.login}
            className="font-medium text-foreground underline underline-offset-4"
          >
            Log in
          </Link>
        </>
      }
    >
      <div className="grid gap-2">
        <Label htmlFor="full_name">Name</Label>
        <Input id="full_name" name="full_name" autoComplete="name" required />
      </div>
      <div className="grid gap-2">
        <Label htmlFor="email">Email</Label>
        <Input id="email" name="email" type="email" autoComplete="email" required />
      </div>
      <div className="grid gap-2">
        <Label htmlFor="password">Password</Label>
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
      <ConsentNotice />
      <Button className="mt-2 w-full" disabled={registerMutation.isPending}>
        {registerMutation.isPending ? <Loader2 className="animate-spin" /> : null}
        {registerMutation.isPending ? "Creating account" : "Create account"}
      </Button>
    </AuthCard>
  );
}
