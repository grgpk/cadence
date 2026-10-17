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
import { PasswordInput } from "../shared/auth-card";
import { Field } from "../shared/field";

const ID = "account";
const MIN_PASSWORD = 8;
const PASSWORD_ERRORS = {
  400: "The current password is wrong, or the new one is too short.",
};

/** The host's own name and password, saved together with one button, apart from the booking page settings. */
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

  const trimmed = name.trim();
  const nameChanged = trimmed !== "" && trimmed !== user.full_name;
  const passwordStarted = current !== "" || next !== "";
  const passwordReady = current !== "" && next.length >= MIN_PASSWORD;
  const pending = saveName.isPending || changePassword.isPending;
  const canSave =
    !pending &&
    trimmed !== "" &&
    (nameChanged || passwordStarted) &&
    (!passwordStarted || passwordReady);

  async function save() {
    try {
      if (nameChanged) await saveName.mutateAsync();
      if (passwordStarted) await changePassword.mutateAsync();
    } catch {
      // Each mutation reports its own failure.
    }
  }

  return (
    <form
      className="grid gap-5"
      onSubmit={(event) => {
        event.preventDefault();
        void save();
      }}
    >
      <Field
        id={`${ID}-name`}
        label="Your name"
        help="Shown on your booking page and in emails."
      >
        <Input
          id={`${ID}-name`}
          autoComplete="name"
          required
          maxLength={120}
          value={name}
          onChange={(event) => setName(event.target.value)}
        />
      </Field>

      <Field id={`${ID}-email`} label="Email" help="The address you log in with.">
        <Input id={`${ID}-email`} value={user.email} readOnly className="bg-muted" />
      </Field>

      <div className="grid items-start gap-4 border-t pt-5 sm:grid-cols-2">
        <Field
          id={`${ID}-current`}
          label="Current password"
          help="Leave both blank to keep your password."
        >
          <PasswordInput
            id={`${ID}-current`}
            autoComplete="current-password"
            required={passwordStarted}
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
            required={passwordStarted}
            minLength={MIN_PASSWORD}
            value={next}
            onChange={(event) => setNext(event.target.value)}
          />
        </Field>
      </div>

      <Button type="submit" className="justify-self-start" disabled={!canSave}>
        {pending ? <Loader2 className="animate-spin" /> : null}
        {pending ? "Saving" : "Save changes"}
      </Button>
    </form>
  );
}
