import { useQuery } from "@tanstack/react-query";
import { createFileRoute } from "@tanstack/react-router";
import { PackageSearch, Search } from "lucide-react";
import { useState } from "react";

import { Input } from "../../../components/ui/input";
import { hostSettingsQuery } from "../../../domains/dashboard/host-api";
import { AppCard, CategoryFilter, SoonRow } from "../../../domains/integrations/app-card";
import {
  APP_CATEGORY,
  type AppCategory,
  type AppEntry,
  APPS,
  appState,
  filterApps,
} from "../../../domains/integrations/catalog";
import { webhooksQuery } from "../../../domains/integrations/webhooks-api";
import { EmptyState } from "../../../domains/shared/empty-state";
import { PageHeader } from "../../../domains/shared/page-header";
import { CONFERENCING_APP } from "../../../domains/integrations/conferencing";
import { hostPageQuery } from "../../../domains/widget/cadence-api";
import { APP_NAME } from "../../../lib/config";

const CATEGORIES = Object.values(APP_CATEGORY);

function isCategory(value: unknown): value is AppCategory {
  return CATEGORIES.includes(value as AppCategory);
}

export const Route = createFileRoute("/dashboard/apps/")({
  validateSearch: (search): { category?: AppCategory } =>
    isCategory(search.category) ? { category: search.category } : {},
  head: () => ({ meta: [{ title: `App store | ${APP_NAME}` }] }),
  component: AppStorePage,
});

function AppGrid({
  title,
  apps,
  stateOf,
}: {
  title: string;
  apps: AppEntry[];
  stateOf: (app: AppEntry) => ReturnType<typeof appState>;
}) {
  if (apps.length === 0) return null;
  return (
    <section className="grid gap-3">
      <h2 className="text-lg font-semibold">{title}</h2>
      <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
        {apps.map((app) => (
          <AppCard key={app.slug} app={app} state={stateOf(app)} />
        ))}
      </div>
    </section>
  );
}

function AppStorePage() {
  const { category } = Route.useSearch();
  const navigate = Route.useNavigate();
  const [search, setSearch] = useState("");
  const settings = useQuery(hostSettingsQuery);
  const slug = settings.data?.public_slug;
  const page = useQuery({ ...hostPageQuery(slug ?? ""), enabled: Boolean(slug) });
  const webhooks = useQuery(webhooksQuery);

  const apps = filterApps(APPS, search, category);
  const stateOf = (app: AppEntry) =>
    appState(
      app.slug,
      settings.data ?? {
        conferencing_app: CONFERENCING_APP.customLink,
        meeting_link: null,
      },
      page.data?.calendar_connected ?? false,
      webhooks.data?.length,
    );

  return (
    <>
      <PageHeader
        title="App store"
        description="Connect your calendar and choose where your calls happen."
        actions={
          <div className="relative w-full sm:w-72">
            <Search className="pointer-events-none absolute top-1/2 left-2.5 size-4 -translate-y-1/2 text-muted-foreground" />
            <Input
              type="search"
              aria-label="Search apps"
              placeholder="Search"
              value={search}
              onChange={(event) => setSearch(event.target.value)}
              className="bg-card pl-8"
            />
          </div>
        }
      />

      <CategoryFilter
        selected={category}
        countOf={(each) =>
          each ? APPS.filter((app) => app.category === each).length : APPS.length
        }
        onSelect={(each) =>
          void navigate({ search: each ? { category: each } : {}, replace: true })
        }
      />

      <AppGrid
        title="Available"
        apps={apps.filter((app) => app.available)}
        stateOf={stateOf}
      />
      {apps.some((app) => !app.available) ? (
        <section className="grid gap-3">
          <h2 className="text-lg font-semibold">Coming soon</h2>
          <ul className="grid gap-3 sm:grid-cols-2">
            {apps
              .filter((app) => !app.available)
              .map((app) => (
                <SoonRow key={app.slug} app={app} />
              ))}
          </ul>
        </section>
      ) : null}
      {apps.length === 0 ? (
        <EmptyState
          icon={PackageSearch}
          title={`No apps match "${search.trim()}"`}
          description="Search looks at app names and what they do. Check the spelling, or clear the search."
        />
      ) : null}
    </>
  );
}
