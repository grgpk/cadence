import { useMutation } from "@tanstack/react-query";
import { useRouter } from "@tanstack/react-router";
import { Loader2 } from "lucide-react";
import { useState } from "react";
import { toast } from "sonner";

import type { ChangePasswordRequest } from "../../bindings/ChangePasswordRequest";
import type { SessionUser } from "../../bindings/SessionUser";
import type { UpdateProfileRequest } from "../../bindings/UpdateProfileRequest";
import { Button } from "../../components/ui/button";
import { Input } from "../../components/ui/input";
import { api } from "../../lib/api";
import { errorMessage } from "../../lib/error-message";
import { PasswordInput } from "../shared/AuthCard";
import { Field } from "../shared/Field";

const ID = "account";
const MIN_PASSWORD = 8;
const PASSWORD_ERRORS = {
  400: "The current password is wrong, or the new one is too short.",
};

/** The host's own name and password. Saved on its own, apart from the booking page settings. */
export function AccountSection({ user }: { user: SessionUser }) {
  const router = useRouter();
  const [name, setName] = useState(user.full_name);
  const [current, setCurrent] = useState("");
  const [next, setNext] = useState("");

  const saveName = useMutation({
    mutationFn: () =>
      api<SessionUser>("/api/auth/me", {
        method: "PUT",
        body: JSON.stringify({ full_name: name.trim() } satisfies UpdateProfileRequest),
      }),
    onSuccess: async () => {
      // The name in the sidebar comes from the session the routes loaded.
      await router.invalidate();
      toast.success("Name saved");
    },
    onError: (error) => toast.error(`Your name was not saved. ${errorMessage(error)}`),
  });
  const changePassword = useMutation({
    mutationFn: () =>
      api<void>("/api/auth/password", {
        method: "POST",
        body: JSON.stringify({
          current_password: current,
          new_password: next,
        } satisfies ChangePasswordRequest),
      }),
    onSuccess: () => {
      setCurrent("");
      setNext("");
      toast.success("Password changed");
    },
    onError: (error) =>
      toast.error(
        `Your password was not changed. ${errorMessage(error, PASSWORD_ERRORS)}`,
      ),
  });

  return (
    <>
      <form
        className="grid gap-3"
        onSubmit={(event) => {
          event.preventDefault();
          saveName.mutate();
        }}
      >
        <Field
          id={`${ID}-name`}
          label="Your name"
          help="Shown on your booking page and in emails."
        >
          <div className="flex flex-wrap gap-2">
            <Input
              id={`${ID}-name`}
              autoComplete="name"
              required
              maxLength={120}
              value={name}
              onChange={(event) => setName(event.target.value)}
              className="min-w-48 flex-1"
            />
            <Button
              type="submit"
              variant="outline"
              disabled={
                saveName.isPending || name.trim() === "" || name.trim() === user.full_name
              }
            >
              {saveName.isPending ? <Loader2 className="animate-spin" /> : null}
              {saveName.isPending ? "Saving" : "Save name"}
            </Button>
          </div>
        </Field>
      </form>

      <Field id={`${ID}-email`} label="Email" help="The address you log in with.">
        <Input id={`${ID}-email`} value={user.email} readOnly className="bg-muted" />
      </Field>

      <form
        className="grid gap-4 border-t pt-5"
        onSubmit={(event) => {
          event.preventDefault();
          changePassword.mutate();
        }}
      >
        <div className="grid items-start gap-4 sm:grid-cols-2">
          <Field id={`${ID}-current`} label="Current password">
            <PasswordInput
              id={`${ID}-current`}
              autoComplete="current-password"
              required
              value={current}
              onChange={(event) => setCurrent(event.target.value)}
            />
          </Field>
          <Field
            id={`${ID}-new`}
            label="New password"
            help={`At least ${MIN_PASSWORD} characters.`}
          >
            <PasswordInput
              id={`${ID}-new`}
              autoComplete="new-password"
              required
              minLength={MIN_PASSWORD}
              value={next}
              onChange={(event) => setNext(event.target.value)}
            />
          </Field>
        </div>
        <Button
          type="submit"
          variant="outline"
          className="justify-self-start"
          disabled={
            changePassword.isPending || current === "" || next.length < MIN_PASSWORD
          }
        >
          {changePassword.isPending ? <Loader2 className="animate-spin" /> : null}
          {changePassword.isPending ? "Changing" : "Change password"}
        </Button>
      </form>
    </>
  );
}
