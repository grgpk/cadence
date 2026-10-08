import { createFileRoute, notFound } from "@tanstack/react-router";

import { getBookingPage } from "../../domains/widget/cadence-api";
import { PublicBookingPage } from "../../domains/widget/PublicBookingPage";
import { APP_NAME } from "../../lib/config";
import { getVisitorCountry } from "../../lib/visitor";

export const Route = createFileRoute("/book/$slug")({
  loader: async ({ params }) => {
    const [country, page] = await Promise.all([
      getVisitorCountry(),
      getBookingPage({ data: params.slug }),
    ]);
    if (!page) throw notFound();
    return { country, page };
  },
  head: ({ loaderData }) => ({
    meta: loaderData
      ? [
          { title: `${loaderData.page.title} | ${loaderData.page.host_name}` },
          { name: "description", content: loaderData.page.description },
        ]
      : [{ title: APP_NAME }],
  }),
  component: HostBookingPage,
});

function HostBookingPage() {
  const { country, page } = Route.useLoaderData();
  const { slug } = Route.useParams();
  return <PublicBookingPage slug={slug} page={page} initialCountryAlpha2={country} />;
}
