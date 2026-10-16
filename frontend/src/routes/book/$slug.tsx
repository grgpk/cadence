import { createFileRoute, notFound } from "@tanstack/react-router";

import { RouteError } from "../../domains/shared/not-found";
import { getBookingPage, getHostPage } from "../../domains/widget/cadence-api";
import {
  HostTypesPage,
  PublicBookingPage,
} from "../../domains/widget/public-booking-page";
import {
  DEFAULT_SLOT_LAYOUT,
  slotViewOf,
  slotViewSearch,
} from "../../domains/widget/slot-days";
import { APP_NAME } from "../../lib/config";
import { getVisitorCountry } from "../../lib/visitor";

export const Route = createFileRoute("/book/$slug")({
  validateSearch: slotViewSearch,
  loader: async ({ params }) => {
    const [country, host] = await Promise.all([
      getVisitorCountry(),
      getHostPage({ data: params.slug }),
    ]);
    if (!host) throw notFound();
    // With one kind of call the link opens it, as every link did before
    // there were several.
    const only = host.types.length === 1 ? host.types[0] : undefined;
    const page = only
      ? await getBookingPage({ data: { host: params.slug, type: only.slug } })
      : null;
    return { country, host, page };
  },
  head: ({ loaderData }) => {
    if (!loaderData) return { meta: [{ title: APP_NAME }] };
    const { host, page } = loaderData;
    const title = page
      ? `${page.title} | ${host.host_name}`
      : `${host.host_name} | ${APP_NAME}`;
    const description = page?.description || `Book a call with ${host.host_name}.`;
    return {
      meta: [
        { title },
        { name: "description", content: description },
        { property: "og:title", content: title },
        { property: "og:description", content: description },
      ],
    };
  },
  component: HostLinkPage,
  errorComponent: RouteError,
});

function HostLinkPage() {
  const { country, host, page } = Route.useLoaderData();
  const { slug } = Route.useParams();
  const { view = DEFAULT_SLOT_LAYOUT } = Route.useSearch();
  const navigate = Route.useNavigate();
  if (!page) return <HostTypesPage slug={slug} host={host} />;
  return (
    <PublicBookingPage
      pageRef={{ host: slug, type: page.event_slug }}
      page={page}
      initialCountryAlpha2={country}
      layout={view}
      onLayoutChange={(next) =>
        void navigate({ search: slotViewOf(next), replace: true })
      }
    />
  );
}
