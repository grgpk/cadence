import { createFileRoute } from "@tanstack/react-router";

import { ManageBookingPage } from "../../domains/bookings/manage-booking-page";
import { RouteError } from "../../domains/shared/not-found";
import { APP_NAME, NOINDEX_META } from "../../lib/config";

export const Route = createFileRoute("/booking/$token")({
  // `?attend=yes` is the "I'm coming" button in the day-before reminder.
  validateSearch: (search): { attend?: "yes" } =>
    search.attend === "yes" ? { attend: "yes" } : {},
  head: () => ({ meta: [{ title: `Your call | ${APP_NAME}` }, NOINDEX_META] }),
  component: ManageRoute,
  errorComponent: RouteError,
});

function ManageRoute() {
  const { token } = Route.useParams();
  const { attend } = Route.useSearch();
  return <ManageBookingPage token={token} attend={attend === "yes"} />;
}
