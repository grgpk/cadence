import { createFileRoute } from "@tanstack/react-router";

import { PublicBookingPage } from "../domains/widget/PublicBookingPage";
import { getVisitorCountry } from "../lib/visitor";

export const Route = createFileRoute("/book-a-call")({
  loader: () => getVisitorCountry(),
  component: BookACallPage,
});

function BookACallPage() {
  return <PublicBookingPage initialCountryAlpha2={Route.useLoaderData()} />;
}
