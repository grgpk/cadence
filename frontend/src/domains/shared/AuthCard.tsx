import { Link } from "@tanstack/react-router";
import { CircleAlert, Eye, EyeOff } from "lucide-react";
import { useState, type ComponentProps, type FormEvent, type ReactNode } from "react";

import { Alert, AlertDescription } from "../../components/ui/alert";
import { Input } from "../../components/ui/input";
import { APP_NAME } from "../../lib/config";
import { errorMessage } from "../../lib/error-message";
import { ROUTES } from "../../lib/routes";
import { Logo } from "./Logo";

export function AuthCard({
  title,
  description,
  error,
  errorMessages,
  onSubmit,
  children,
  footer,
}: {
  title: string;
  description: string;
  error: unknown;
  /** What a failed status means on this form, such as a wrong password. */
  errorMessages?: Partial<Record<number, string>>;
  onSubmit: (event: FormEvent<HTMLFormElement>) => void;
  children: ReactNode;
  footer: ReactNode;
}) {
  return (
    <main className="flex min-h-svh flex-col items-center justify-center gap-8 p-4 sm:p-6">
      <Link to={ROUTES.home} aria-label={`${APP_NAME} home`}>
        <Logo />
      </Link>
      <div className="w-full max-w-sm rounded-xl border bg-card p-6 sm:p-8">
        <h1 className="font-display text-2xl font-semibold tracking-tight">{title}</h1>
        <p className="mt-1 text-sm text-muted-foreground">{description}</p>
        <form onSubmit={onSubmit} className="mt-6 grid gap-4">
          {error ? (
            <Alert variant="destructive">
              <CircleAlert className="size-4" />
              <AlertDescription>{errorMessage(error, errorMessages)}</AlertDescription>
            </Alert>
          ) : null}
          {children}
        </form>
      </div>
      <p className="text-sm text-muted-foreground">{footer}</p>
    </main>
  );
}

export function PasswordInput(props: Omit<ComponentProps<typeof Input>, "type">) {
  const [visible, setVisible] = useState(false);

  return (
    <div className="relative">
      <Input {...props} type={visible ? "text" : "password"} className="pr-9" />
      <button
        type="button"
        aria-label={visible ? "Hide password" : "Show password"}
        aria-pressed={visible}
        onClick={() => setVisible(!visible)}
        className="absolute top-1/2 right-1.5 -translate-y-1/2 rounded-sm p-1 text-muted-foreground outline-none hover:text-foreground focus-visible:ring-[3px] focus-visible:ring-ring/50"
      >
        {visible ? <EyeOff className="size-4" /> : <Eye className="size-4" />}
      </button>
    </div>
  );
}
