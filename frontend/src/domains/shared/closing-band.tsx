import { Link } from "@tanstack/react-router";

import { Button } from "../../components/ui/button";
import { useSessionUser } from "../../lib/auth";
import { ROUTES } from "../../lib/routes";

/**
 * The last word on a public page, with the sign-up for visitors. Ink on a
 * light page; on a dark page a dusk-tinted night, so it never flashes white.
 */
export function ClosingBand({ title }: { title: string }) {
  const signedIn = useSessionUser() !== null;
  return (
    <section className="border-t bg-foreground text-background dark:bg-dusk/15 dark:text-foreground">
      <div className="mx-auto flex max-w-[82rem] flex-wrap items-center justify-between gap-x-8 gap-y-6 px-4 py-16 sm:px-6 sm:py-24">
        <h2 className="max-w-2xl font-display font-wide text-3xl leading-[1.05] font-bold tracking-tight sm:text-4xl">
          {title}
        </h2>
        {signedIn ? null : (
          <Button
            size="lg"
            variant="secondary"
            className="dark:bg-primary dark:text-primary-foreground dark:hover:bg-primary/90"
            asChild
          >
            <Link to={ROUTES.register}>Create your account</Link>
          </Button>
        )}
      </div>
    </section>
  );
}
