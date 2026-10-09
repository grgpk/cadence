import { Link } from "@tanstack/react-router";
import { CircleAlert, Eye, EyeOff } from "lucide-react";
import { useState, type ComponentProps, type FormEvent, type ReactNode } from "react";

import { Alert, AlertDescription } from "../../components/ui/alert";
import { Input } from "../../components/ui/input";
import { APP_NAME } from "../../lib/config";
import { errorMessage } from "../../lib/error-message";
import { ROUTES } from "../../lib/routes";
import { DayTape } from "../landing/DayTape";
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
    <div className="grid min-h-svh lg:grid-cols-2">
      <main className="flex flex-col gap-10 p-6 sm:p-10">
        <Link to={ROUTES.home} aria-label={`${APP_NAME} home`} className="self-start">
          <Logo />
        </Link>
        <div className="mx-auto flex w-full max-w-sm flex-1 flex-col justify-center">
          <h1 className="font-display font-wide text-3xl font-bold tracking-tight">
            {title}
          </h1>
          <p className="mt-1.5 text-muted-foreground">{description}</p>
          <form onSubmit={onSubmit} className="mt-8 grid gap-4">
            {error ? (
              <Alert variant="destructive">
                <CircleAlert className="size-4" />
                <AlertDescription>{errorMessage(error, errorMessages)}</AlertDescription>
              </Alert>
            ) : null}
            {children}
          </form>
          <p className="mt-6 text-sm text-muted-foreground">{footer}</p>
        </div>
      </main>
      {/* Always the night side: a nested dark scope reuses the dark tokens. */}
      <aside
        aria-hidden="true"
        className="dark hidden flex-col justify-center gap-12 overflow-hidden border-l bg-card py-10 text-foreground lg:flex"
      >
        <p className="max-w-md px-10 font-display font-wide text-4xl leading-[1.05] font-bold tracking-tight xl:text-5xl">
          Set your hours once. Share one link.
        </p>
        <DayTape />
      </aside>
    </div>
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
