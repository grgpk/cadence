import { createFileRoute } from "@tanstack/react-router";

import { getPlans } from "../domains/pricing/pricing-api";
import { PricingPage } from "../domains/pricing/pricing-page";
import { RouteError } from "../domains/shared/not-found";
import { APP_NAME } from "../lib/config";

const TITLE = `Pricing | ${APP_NAME}`;
const DESCRIPTION =
  "Free for a few calls a week. Pro and Business for unlimited bookings, reminders and your own brand. Try Pro free for 14 days.";

export const Route = createFileRoute("/pricing")({
  loader: () => getPlans(),
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
  return <PricingPage offers={Route.useLoaderData()} />;
}
