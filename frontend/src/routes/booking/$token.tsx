import { createFileRoute } from "@tanstack/react-router";

import { ManageBookingPage } from "../../domains/bookings/ManageBookingPage";
import { RouteError } from "../../domains/shared/NotFound";
import { APP_NAME, NOINDEX_META } from "../../lib/config";

export const Route = createFileRoute("/booking/$token")({
  head: () => ({ meta: [{ title: `Your call | ${APP_NAME}` }, NOINDEX_META] }),
  component: ManageRoute,
  errorComponent: RouteError,
});

function ManageRoute() {
  const { token } = Route.useParams();
  return <ManageBookingPage token={token} />;
}
