import { Link } from "@tanstack/react-router";
import { type ReactNode, useEffect, useState } from "react";

import { cn } from "cn";

import { Button } from "../../components/ui/button";
import { useSessionUser } from "../../lib/auth";
import { APP_NAME } from "../../lib/config";
import { ROUTES } from "../../lib/routes";
import { HOW_IT_WORKS_ID } from "../landing/section-ids";
import { Logo } from "./logo";
import { ThemeToggle } from "./theme-toggle";

/** True once the page has scrolled, so the header can lift off the content. */
function useScrolled(): boolean {
  const [scrolled, setScrolled] = useState(false);
  useEffect(() => {
    const update = () => setScrolled(window.scrollY > 4);
    update();
    window.addEventListener("scroll", update, { passive: true });
    return () => window.removeEventListener("scroll", update);
  }, []);
  return scrolled;
}

const FOOTER_LINK = "text-muted-foreground hover:text-foreground";

export function PublicLayout({
  children,
  accountLinks = true,
}: {
  children: ReactNode;
  /** Off on pages an invitee lands on: logging in is for hosts. */
  accountLinks?: boolean;
}) {
  const user = useSessionUser();
  const scrolled = useScrolled();
  return (
    <div className="flex min-h-svh flex-col">
      <header
        className={cn(
          "sticky top-0 z-20 border-b transition-[background-color,border-color] duration-200",
          scrolled ? "bg-background/85 backdrop-blur" : "border-transparent",
        )}
      >
        <div className="mx-auto flex h-14 max-w-[82rem] items-center justify-between gap-4 px-4 sm:px-6">
          <Link to={ROUTES.home} aria-label={`${APP_NAME} home`}>
            <Logo />
          </Link>
          <nav className="flex items-center gap-1">
            {accountLinks ? (
              <>
                <Button variant="ghost" size="sm" className="max-sm:hidden" asChild>
                  <Link to={ROUTES.home} hash={HOW_IT_WORKS_ID}>
                    How it works
                  </Link>
                </Button>
                <Button variant="ghost" size="sm" asChild>
                  <Link to={ROUTES.pricing}>Pricing</Link>
                </Button>
              </>
            ) : null}
            <ThemeToggle />
            {!accountLinks ? null : user ? (
              <Button size="sm" asChild>
                <Link to={ROUTES.dashboard}>Go to dashboard</Link>
              </Button>
            ) : (
              <>
                <Button variant="ghost" size="sm" asChild>
                  <Link to={ROUTES.login}>Log in</Link>
                </Button>
                <Button size="sm" asChild>
                  <Link to={ROUTES.register}>Sign up</Link>
                </Button>
              </>
            )}
          </nav>
        </div>
      </header>
      <main className="flex-1">{children}</main>
      <footer className="border-t">
        <div className="mx-auto grid max-w-[82rem] gap-10 px-4 py-12 text-sm sm:grid-cols-[minmax(0,1fr)_auto_auto] sm:gap-16 sm:px-6">
          <div>
            <Logo />
            <p className="mt-3 max-w-xs text-muted-foreground">
              One link for people to book a call with you.
            </p>
            <p className="mt-6 text-muted-foreground">
              © {new Date().getFullYear()} {APP_NAME}
            </p>
          </div>
          <nav aria-label="Product" className="grid content-start gap-2.5">
            <p className="font-semibold">Product</p>
            <Link to={ROUTES.home} hash={HOW_IT_WORKS_ID} className={FOOTER_LINK}>
              How it works
            </Link>
            <Link to={ROUTES.pricing} className={FOOTER_LINK}>
              Pricing
            </Link>
            {user ? (
              <Link to={ROUTES.dashboard} className={FOOTER_LINK}>
                Dashboard
              </Link>
            ) : (
              <>
                <Link to={ROUTES.register} className={FOOTER_LINK}>
                  Sign up
                </Link>
                <Link to={ROUTES.login} className={FOOTER_LINK}>
                  Log in
                </Link>
              </>
            )}
          </nav>
          <nav aria-label="Legal" className="grid content-start gap-2.5">
            <p className="font-semibold">Legal</p>
            <Link to={ROUTES.terms} className={FOOTER_LINK}>
              Terms
            </Link>
            <Link to={ROUTES.privacy} className={FOOTER_LINK}>
              Privacy
            </Link>
          </nav>
        </div>
      </footer>
    </div>
  );
}
