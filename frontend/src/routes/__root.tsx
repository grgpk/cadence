import {
  createRootRoute,
  HeadContent,
  Outlet,
  Scripts,
  redirect,
} from "@tanstack/react-router";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { useState } from "react";
import { getSessionUser, PUBLIC_PATHS } from "../lib/auth";
import { APP_NAME } from "../lib/config";
import { ROUTES } from "../lib/routes";
import { THEME_INIT_SCRIPT } from "../lib/theme";
import { Toaster } from "../components/ui/sonner";
import { TooltipProvider } from "../components/ui/tooltip";
import { NotFound, RouteError } from "../domains/shared/NotFound";
import "../styles/globals.css";

export const Route = createRootRoute({
  beforeLoad: async ({ location }) => {
    if (PUBLIC_PATHS.includes(location.pathname)) return { user: null };
    const user = await getSessionUser();
    if (!user) throw redirect({ to: ROUTES.login });
    return { user };
  },
  pendingMs: 0,
  pendingComponent: () => null,
  head: () => ({
    meta: [
      { charSet: "utf-8" },
      { name: "viewport", content: "width=device-width, initial-scale=1" },
      { title: APP_NAME },
      {
        name: "description",
        content: "Share one link and let people book a call at a time that works.",
      },
    ],
    links: [{ rel: "icon", href: "/favicon.svg" }],
  }),
  shellComponent: RootDocument,
  component: () => <Outlet />,
  notFoundComponent: NotFound,
  errorComponent: RouteError,
});

function RootDocument({ children }: { children: React.ReactNode }) {
  const [queryClient] = useState(() => new QueryClient());

  return (
    // The theme script sets the class before hydration, so the attribute may differ.
    <html lang="en" suppressHydrationWarning>
      <head>
        <HeadContent />
        <script dangerouslySetInnerHTML={{ __html: THEME_INIT_SCRIPT }} />
      </head>
      <body>
        <QueryClientProvider client={queryClient}>
          <TooltipProvider>{children}</TooltipProvider>
          <Toaster position="bottom-right" />
        </QueryClientProvider>
        <Scripts />
      </body>
    </html>
  );
}
