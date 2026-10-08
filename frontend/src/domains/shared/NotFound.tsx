import { Link } from "@tanstack/react-router";

import { Button } from "../../components/ui/button";
import { ROUTES } from "../../lib/routes";
import { Logo } from "./Logo";

export function NotFound() {
  return (
    <main className="flex min-h-svh flex-col items-center justify-center gap-6 p-6 text-center">
      <Logo />
      <div>
        <h1 className="font-display text-3xl font-semibold tracking-tight">
          This page does not exist
        </h1>
        <p className="mt-2 text-muted-foreground">
          The link may be old, or the address may have a typo.
        </p>
      </div>
      <Button asChild>
        <Link to={ROUTES.home}>Go to the booking page</Link>
      </Button>
    </main>
  );
}

export function RouteError({ error, reset }: { error: unknown; reset: () => void }) {
  return (
    <main className="flex min-h-svh flex-col items-center justify-center gap-6 p-6 text-center">
      <Logo />
      <div>
        <h1 className="font-display text-3xl font-semibold tracking-tight">
          This page failed to load
        </h1>
        <p className="mt-2 text-muted-foreground">
          {error instanceof Error ? error.message : "An unexpected error occurred."}
        </p>
      </div>
      <Button onClick={reset}>Try again</Button>
    </main>
  );
}
