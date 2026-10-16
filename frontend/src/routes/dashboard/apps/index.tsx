import { useQuery } from "@tanstack/react-query";
import { createFileRoute } from "@tanstack/react-router";
import { PackageSearch, Search } from "lucide-react";
import { useState } from "react";

import { Input } from "../../../components/ui/input";
import { hostSettingsQuery } from "../../../domains/dashboard/host-api";
import { AppCard, CategoryTile, SoonRow } from "../../../domains/integrations/app-card";
import {
  APP_CATEGORY,
  type AppCategory,
  type AppEntry,
  APPS,
  filterApps,
  isInstalled,
} from "../../../domains/integrations/catalog";
import { EmptyState } from "../../../domains/shared/empty-state";
import { PageHeader } from "../../../domains/shared/page-header";
import { bookingPageQuery } from "../../../domains/widget/cadence-api";
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
  installed,
}: {
  title: string;
  apps: AppEntry[];
  installed: (app: AppEntry) => boolean;
}) {
  if (apps.length === 0) return null;
  return (
    <section className="grid gap-3">
      <h2 className="text-lg font-semibold">{title}</h2>
      <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
        {apps.map((app) => (
          <AppCard key={app.slug} app={app} installed={installed(app)} />
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
  const page = useQuery({ ...bookingPageQuery(slug ?? ""), enabled: Boolean(slug) });

  const apps = filterApps(APPS, search, category);
  const installed = (app: AppEntry) =>
    settings.data
      ? isInstalled(app.slug, settings.data, page.data?.calendar_connected ?? false)
      : false;

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

      <section className="grid gap-3">
        <h2 className="text-lg font-semibold">Categories</h2>
        <div className="grid gap-4 sm:grid-cols-2">
          {CATEGORIES.map((each) => (
            <CategoryTile
              key={each}
              category={each}
              count={APPS.filter((app) => app.category === each).length}
              selected={category === each}
              onToggle={() =>
                void navigate({
                  search: category === each ? {} : { category: each },
                  replace: true,
                })
              }
            />
          ))}
        </div>
      </section>

      <AppGrid
        title="Available"
        apps={apps.filter((app) => app.available)}
        installed={installed}
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
