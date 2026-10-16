import { createFileRoute, notFound } from "@tanstack/react-router";

import { RouteError } from "../../domains/shared/not-found";
import { getBookingPage } from "../../domains/widget/cadence-api";
import { PublicBookingPage } from "../../domains/widget/public-booking-page";
import {
  DEFAULT_SLOT_LAYOUT,
  isSlotLayout,
  type SlotLayout,
} from "../../domains/widget/slot-days";
import { APP_NAME } from "../../lib/config";
import { getVisitorCountry } from "../../lib/visitor";

export const Route = createFileRoute("/book/$slug")({
  // `?view=week` or `?view=list` opens the page on that layout; month needs no value.
  validateSearch: (search): { view?: SlotLayout } =>
    isSlotLayout(search.view) && search.view !== DEFAULT_SLOT_LAYOUT
      ? { view: search.view }
      : {},
  loader: async ({ params }) => {
    const [country, page] = await Promise.all([
      getVisitorCountry(),
      getBookingPage({ data: params.slug }),
    ]);
    if (!page) throw notFound();
    return { country, page };
  },
  head: ({ loaderData }) => {
    if (!loaderData) return { meta: [{ title: APP_NAME }] };
    const { page } = loaderData;
    const title = `${page.title} | ${page.host_name}`;
    // The description is optional in a host's settings.
    const description = page.description || `Book a call with ${page.host_name}.`;
    return {
      meta: [
        { title },
        { name: "description", content: description },
        { property: "og:title", content: title },
        { property: "og:description", content: description },
      ],
    };
  },
  component: HostBookingPage,
  errorComponent: RouteError,
});

function HostBookingPage() {
  const { country, page } = Route.useLoaderData();
  const { slug } = Route.useParams();
  const { view = DEFAULT_SLOT_LAYOUT } = Route.useSearch();
  const navigate = Route.useNavigate();
  return (
    <PublicBookingPage
      slug={slug}
      page={page}
      initialCountryAlpha2={country}
      layout={view}
      onLayoutChange={(next) =>
        void navigate({
          search: next === DEFAULT_SLOT_LAYOUT ? {} : { view: next },
          replace: true,
        })
      }
    />
  );
}
