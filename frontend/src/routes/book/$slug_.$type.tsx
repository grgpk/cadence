import { createFileRoute, notFound } from "@tanstack/react-router";

import { RouteError } from "../../domains/shared/not-found";
import { getBookingPage, previewMeta } from "../../domains/widget/cadence-api";
import { PublicBookingPage } from "../../domains/widget/public-booking-page";
import {
  DEFAULT_SLOT_LAYOUT,
  slotViewOf,
  slotViewSearch,
} from "../../domains/widget/slot-days";
import { APP_NAME } from "../../lib/config";
import { getVisitorCountry } from "../../lib/visitor";

/** One kind of call on a host's link: `/book/<host>/<type>`. */
export const Route = createFileRoute("/book/$slug_/$type")({
  validateSearch: slotViewSearch,
  loader: async ({ params }) => {
    const [country, page] = await Promise.all([
      getVisitorCountry(),
      getBookingPage({ data: { host: params.slug, type: params.type } }),
    ]);
    if (!page) throw notFound();
    return { country, page };
  },
  head: ({ loaderData, params }) => {
    if (!loaderData) return { meta: [{ title: APP_NAME }] };
    const { page } = loaderData;
    const title = `${page.title} | ${page.host_name}`;
    const description = page.description || `Book a call with ${page.host_name}.`;
    return {
      meta: [
        { title },
        { name: "description", content: description },
        { property: "og:title", content: title },
        { property: "og:description", content: description },
        ...previewMeta(params.slug),
      ],
    };
  },
  component: TypeBookingPage,
  errorComponent: RouteError,
});

function TypeBookingPage() {
  const { country, page } = Route.useLoaderData();
  const { slug, type } = Route.useParams();
  const { view = DEFAULT_SLOT_LAYOUT } = Route.useSearch();
  const navigate = Route.useNavigate();
  return (
    <PublicBookingPage
      pageRef={{ host: slug, type }}
      page={page}
      initialCountryAlpha2={country}
      layout={view}
      onLayoutChange={(next) =>
        void navigate({ search: slotViewOf(next), replace: true })
      }
    />
  );
}
