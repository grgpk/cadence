import { createFileRoute, useNavigate } from "@tanstack/react-router";

import { getPlans } from "../domains/pricing/pricing-api";
import {
  DEFAULT_INTERVAL,
  needsFrom,
  pricingSearch,
  searchFor,
} from "../domains/pricing/pricing";
import { PricingPage } from "../domains/pricing/pricing-page";
import { RouteError } from "../domains/shared/not-found";
import { APP_NAME } from "../lib/config";

const TITLE = `Pricing | ${APP_NAME}`;
const DESCRIPTION =
  "Free for a few calls a week. Pro and Business for unlimited bookings, reminders and your own brand. Try Pro free for 14 days.";

export const Route = createFileRoute("/pricing")({
  // `?interval=&calls=&kinds=`: the answers survive the back button and a shared link.
  validateSearch: pricingSearch,
  loader: () => getPlans(),
  // The plans change with a deploy; moving the slider must not fetch them again.
  staleTime: Number.POSITIVE_INFINITY,
  head: () => ({
    meta: [
      { title: TITLE },
      { name: "description", content: DESCRIPTION },
      { property: "og:title", content: TITLE },
      { property: "og:description", content: DESCRIPTION },
    ],
  }),
  component: PricingRoute,
  errorComponent: RouteError,
});

function PricingRoute() {
  const search = Route.useSearch();
  const navigate = useNavigate({ from: Route.fullPath });
  return (
    <PricingPage
      offers={Route.useLoaderData()}
      interval={search.interval ?? DEFAULT_INTERVAL}
      needs={needsFrom(search)}
      onChange={(interval, needs) =>
        void navigate({
          search: searchFor(interval, needs),
          replace: true,
          resetScroll: false,
        })
      }
    />
  );
}
