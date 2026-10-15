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
import { THEME_INIT_SCRIPT } from "../hooks/theme";
import { getSiteOrigin } from "../lib/visitor";
import { Toaster } from "../components/ui/sonner";
import { TooltipProvider } from "../components/ui/tooltip";
import { NotFound, RouteError } from "../domains/shared/not-found";
import "../styles/globals.css";

const PREVIEW_IMAGE = "/og.png";

export const Route = createRootRoute({
  beforeLoad: async ({ location }) => {
    if (!isProtectedPath(location.pathname)) return { user: null };
    const user = await getSessionUser();
    if (!user) {
      throw redirect({ to: ROUTES.login, search: { redirect: location.href } });
    }
    return { user };
  },
  // Asked once per visit: the address does not change while someone browses.
  loader: async () => ({ origin: await getSiteOrigin() }),
  staleTime: Number.POSITIVE_INFINITY,
  pendingMs: 0,
  pendingComponent: () => null,
  head: ({ loaderData }) => ({
    meta: [
      { charSet: "utf-8" },
      { name: "viewport", content: "width=device-width, initial-scale=1" },
      { title: APP_NAME },
      { name: "description", content: APP_DESCRIPTION },
      { property: "og:site_name", content: APP_NAME },
      { property: "og:type", content: "website" },
      { property: "og:title", content: APP_NAME },
      { property: "og:description", content: APP_DESCRIPTION },
      // The card shown when a link to any page is pasted into a chat.
      { property: "og:image", content: `${loaderData?.origin ?? ""}${PREVIEW_IMAGE}` },
      { property: "og:image:width", content: "1200" },
      { property: "og:image:height", content: "630" },
      { name: "twitter:card", content: "summary_large_image" },
      { name: "theme-color", content: "#151b3d" },
    ],
    links: [
      { rel: "icon", href: "/favicon.svg" },
      { rel: "apple-touch-icon", href: "/apple-touch-icon.png" },
      { rel: "manifest", href: "/manifest.webmanifest" },
    ],
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
