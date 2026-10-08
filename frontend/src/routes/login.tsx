import { useMutation, useQueryClient } from "@tanstack/react-query";
import { createFileRoute, Link, redirect, useRouter } from "@tanstack/react-router";

import type { LoginRequest } from "../bindings/LoginRequest";
import { Button } from "../components/ui/button";
import { Input } from "../components/ui/input";
import { Label } from "../components/ui/label";
import { AuthCard, PasswordInput } from "../domains/shared/AuthCard";
import { RouteError } from "../domains/shared/NotFound";
import { api } from "../lib/api";
import { getSessionUser } from "../lib/auth";
import { APP_NAME, NOINDEX_META } from "../lib/config";
import { ROUTES, safeReturnPath } from "../lib/routes";

const WRONG_CREDENTIALS = "The email or password is wrong.";
const LOGIN_ERRORS = { 400: WRONG_CREDENTIALS, 401: WRONG_CREDENTIALS };

export const Route = createFileRoute("/login")({
  // The page someone was sent here from, when it is one of ours.
  validateSearch: (search): { redirect?: string } => {
    const redirect = safeReturnPath(search.redirect);
    return redirect ? { redirect } : {};
  },
  beforeLoad: async () => {
    if (await getSessionUser()) throw redirect({ to: ROUTES.dashboard });
  },
  head: () => ({ meta: [{ title: `Log in | ${APP_NAME}` }, NOINDEX_META] }),
  component: LoginPage,
  errorComponent: RouteError,
});

function LoginPage() {
  const router = useRouter();
  const queryClient = useQueryClient();
  const { redirect: returnPath } = Route.useSearch();
  const loginMutation = useMutation({
    mutationFn: (credentials: LoginRequest) =>
      api<void>("/api/auth/login", {
        method: "POST",
        body: JSON.stringify(credentials),
      }),
    onSuccess: () => {
      // Anything cached belongs to whoever used this tab before.
      queryClient.clear();
      router.history.push(returnPath ?? ROUTES.dashboard);
    },
  });

  function submit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const form = new FormData(event.currentTarget);
    loginMutation.mutate({
      email: String(form.get("email") ?? ""),
      password: String(form.get("password") ?? ""),
    });
  }

  return (
    <AuthCard
      title="Log in"
      description="Manage your bookings and the hours you take calls."
      error={loginMutation.error}
      errorMessages={LOGIN_ERRORS}
      onSubmit={submit}
      footer={
        <>
          New to {APP_NAME}?{" "}
          <Link
            to={ROUTES.register}
            className="font-medium text-foreground underline underline-offset-4"
          >
            Create an account
          </Link>
        </>
      }
    >
      <div className="grid gap-2">
        <Label htmlFor="email">Email</Label>
        <Input id="email" name="email" type="email" autoComplete="email" required />
      </div>
      <div className="grid gap-2">
        <Label htmlFor="password">Password</Label>
        <PasswordInput
          id="password"
          name="password"
          autoComplete="current-password"
          required
        />
      </div>
      <Button className="mt-2 w-full" disabled={loginMutation.isPending}>
        {loginMutation.isPending ? "Logging in" : "Log in"}
      </Button>
    </AuthCard>
  );
}
