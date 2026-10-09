import { Link, useRouter } from "@tanstack/react-router";
import { useEffect } from "react";

import { Button } from "../../components/ui/button";
import { reportError } from "../../lib/bug-report";
import { errorMessage } from "../../lib/error-message";
import { ROUTES } from "../../lib/routes";
import { ErrorState } from "./ErrorState";
import { Logo } from "./Logo";

export function NotFound() {
  return (
    <main className="flex min-h-svh flex-col items-center justify-center gap-6 p-6 text-center">
      <Logo />
      <div>
        <h1 className="font-display font-wide text-3xl font-bold tracking-tight">
          This page does not exist
        </h1>
        <p className="mt-2 text-muted-foreground">
          The link may be old, or the address may have a typo.
        </p>
      </div>
      <Button asChild>
        <Link to={ROUTES.home}>Go to the home page</Link>
      </Button>
    </main>
  );
}

type RouteErrorProps = { error: unknown; reset: () => void };

/**
 * Reports what broke the page, and returns the retry. A failed loader needs the
 * route loaded again, not only the boundary reset.
 */
function useRetry(error: unknown, reset: () => void) {
  const router = useRouter();
  // React hands a render error to the boundary, so no window event fires for it.
  useEffect(() => reportError("JsError", error), [error]);
  return () => {
    void router.invalidate();
    reset();
  };
}

/** A failed page with nothing around it: public pages, and failures of the shell itself. */
export function RouteError({ error, reset }: RouteErrorProps) {
  const retry = useRetry(error, reset);
  return (
    <main className="flex min-h-svh flex-col items-center justify-center gap-6 p-6 text-center">
      <Logo />
      <div>
        <h1 className="font-display font-wide text-3xl font-bold tracking-tight">
          This page failed to load
        </h1>
        <p className="mt-2 text-muted-foreground">{errorMessage(error)}</p>
      </div>
      <Button onClick={retry}>Try again</Button>
    </main>
  );
}

/** A failed page inside the app shell, so the navigation stays usable. */
export function PageError({ error, reset }: RouteErrorProps) {
  return (
    <ErrorState
      title="This page failed to load"
      error={error}
      onRetry={useRetry(error, reset)}
    />
  );
}
