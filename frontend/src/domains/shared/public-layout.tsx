import { Link } from "@tanstack/react-router";
import type { ReactNode } from "react";

import { Button } from "../../components/ui/button";
import { APP_NAME } from "../../lib/config";
import { ROUTES } from "../../lib/routes";
import { Logo } from "./logo";
import { ThemeToggle } from "./theme-toggle";

export function PublicLayout({
  children,
  accountLinks = true,
}: {
  children: ReactNode;
  /** Off on pages an invitee lands on: logging in is for hosts. */
  accountLinks?: boolean;
}) {
  return (
    <div className="flex min-h-svh flex-col">
      <header className="sticky top-0 z-20 border-b bg-background/85 backdrop-blur">
        <div className="mx-auto flex h-14 max-w-[82rem] items-center justify-between gap-4 px-4 sm:px-6">
          <Link to={ROUTES.home} aria-label={`${APP_NAME} home`}>
            <Logo />
          </Link>
          <nav className="flex items-center gap-1">
            {accountLinks ? (
              <Button variant="ghost" size="sm" asChild>
                <Link to={ROUTES.pricing}>Pricing</Link>
              </Button>
            ) : null}
            <ThemeToggle />
            {accountLinks ? (
              <>
                <Button variant="ghost" size="sm" asChild>
                  <Link to={ROUTES.login}>Log in</Link>
                </Button>
                <Button size="sm" asChild>
                  <Link to={ROUTES.register}>Sign up</Link>
                </Button>
              </>
            ) : null}
          </nav>
        </div>
      </header>
      <main className="flex-1">{children}</main>
      <footer className="border-t">
        <div className="mx-auto flex max-w-[82rem] flex-col gap-3 px-4 py-6 text-sm text-muted-foreground sm:flex-row sm:items-center sm:justify-between sm:px-6">
          <p>
            © {new Date().getFullYear()} {APP_NAME}
          </p>
          <nav className="flex gap-5">
            <Link to={ROUTES.pricing} className="hover:text-foreground">
              Pricing
            </Link>
            <Link to={ROUTES.terms} className="hover:text-foreground">
              Terms
            </Link>
            <Link to={ROUTES.privacy} className="hover:text-foreground">
              Privacy
            </Link>
          </nav>
        </div>
      </footer>
    </div>
  );
}
