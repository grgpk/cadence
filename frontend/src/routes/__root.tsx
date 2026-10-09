import {
  createRootRoute,
  HeadContent,
  Outlet,
  Scripts,
  redirect,
  useRouterState,
} from "@tanstack/react-router";
import { useEffect } from "react";
import { getSessionUser } from "../lib/auth";
import { watchErrors } from "../lib/bug-report";
import { APP_DESCRIPTION, APP_NAME } from "../lib/config";
import { isProtectedPath, ROUTES } from "../lib/routes";
import { THEME_INIT_SCRIPT } from "../lib/theme";
import { Toaster } from "../components/ui/sonner";
import { TooltipProvider } from "../components/ui/tooltip";
import { NotFound, RouteError } from "../domains/shared/NotFound";
import "../styles/globals.css";

export const Route = createRootRoute({
  beforeLoad: async ({ location }) => {
    if (!isProtectedPath(location.pathname)) return { user: null };
    const user = await getSessionUser();
    if (!user) {
      throw redirect({ to: ROUTES.login, search: { redirect: location.href } });
    }
    return { user };
  },
  pendingMs: 0,
  pendingComponent: () => null,
  head: () => ({
    meta: [
      { charSet: "utf-8" },
      { name: "viewport", content: "width=device-width, initial-scale=1" },
      { title: APP_NAME },
      { name: "description", content: APP_DESCRIPTION },
      { property: "og:site_name", content: APP_NAME },
      { property: "og:type", content: "website" },
      { property: "og:title", content: APP_NAME },
      { property: "og:description", content: APP_DESCRIPTION },
      { name: "twitter:card", content: "summary" },
    ],
    links: [{ rel: "icon", href: "/favicon.svg" }],
  }),
  shellComponent: RootDocument,
  component: () => <Outlet />,
  notFoundComponent: NotFound,
  errorComponent: RouteError,
});

/** A line across the top while the next page loads, so a slow one never looks stuck. */
function NavigationProgress() {
  const pending = useRouterState({ select: (state) => state.status === "pending" });
  if (!pending) return null;
  return (
    <div
      role="progressbar"
      aria-label="Loading page"
      className="fixed inset-x-0 top-0 z-50 h-0.5 origin-left animate-nav-progress bg-dusk"
    />
  );
}

function RootDocument({ children }: { children: React.ReactNode }) {
  useEffect(watchErrors, []);

  return (
    // The theme script sets the class before hydration, so the attribute may differ.
    <html lang="en" suppressHydrationWarning>
      <head>
        <HeadContent />
        <script dangerouslySetInnerHTML={{ __html: THEME_INIT_SCRIPT }} />
      </head>
      <body>
        <NavigationProgress />
        <TooltipProvider>{children}</TooltipProvider>
        <Toaster position="bottom-right" />
        <Scripts />
      </body>
    </html>
  );
}
