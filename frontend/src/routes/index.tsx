import { createFileRoute } from "@tanstack/react-router";
import { PublicBookingPage } from "../domains/widget/PublicBookingPage";
import { getVisitorCountry } from "../lib/visitor";

export const Route = createFileRoute("/")({
  loader: () => getVisitorCountry(),
  component: HomeBookingPage,
});

function HomeBookingPage() {
  return <PublicBookingPage initialCountryAlpha2={Route.useLoaderData()} />;
}
