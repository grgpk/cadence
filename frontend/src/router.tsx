import {
  MutationCache,
  QueryCache,
  QueryClient,
  QueryClientProvider,
} from "@tanstack/react-query";
import { createRouter } from "@tanstack/react-router";

import { PageError } from "./domains/shared/NotFound";
import { ApiError } from "./lib/api";
import { isProtectedPath, ROUTES } from "./lib/routes";
import { routeTree } from "./routeTree.gen";

const MAX_RETRIES = 1;
const SERVER_ERROR = 500;

export function getRouter() {
  // A session that ended mid-visit: log in again, then come back to this page.
  // Login answers a wrong password with the same status, so public pages are left alone.
  function onError(error: unknown): void {
    if (!(error instanceof ApiError) || !error.isUnauthorized) return;
    const { pathname, href } = router.state.location;
    if (!isProtectedPath(pathname)) return;
    void router.navigate({ to: ROUTES.login, search: { redirect: href } });
  }

  const queryClient = new QueryClient({
    queryCache: new QueryCache({ onError }),
    mutationCache: new MutationCache({ onError }),
    defaultOptions: {
      queries: {
        // Asking again does not change an answer the API gave on purpose.
        retry: (failures, error) =>
          failures < MAX_RETRIES &&
          !(error instanceof ApiError && error.status < SERVER_ERROR),
      },
    },
  });

  const router = createRouter({
    routeTree,
    defaultPreload: "intent",
    scrollRestoration: true,
    defaultErrorComponent: PageError,
    Wrap: ({ children }) => (
      <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
    ),
  });
  return router;
}

declare module "@tanstack/react-router" {
  interface Register {
    router: ReturnType<typeof getRouter>;
  }
}
