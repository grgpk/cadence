import { useQuery } from "@tanstack/react-query";
import { createFileRoute } from "@tanstack/react-router";
import { cn } from "cn";
import { PackageSearch, Search } from "lucide-react";
import { useState } from "react";

import { Input } from "../../../components/ui/input";
import { hostSettingsQuery } from "../../../domains/dashboard/host-api";
import {
  AppCard,
  CategoryFilter,
  SoonList,
} from "../../../domains/integrations/app-card";
import {
  APP_CATEGORY,
  APP_TONE,
  type AppCategory,
  type AppEntry,
  APPS,
  appState,
  CATEGORY_INFO,
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

/** One category's apps beside what it is for and how much of it the host uses. */
function CategoryShelf({
  category,
  apps,
  stateOf,
}: {
  category: AppCategory;
  apps: AppEntry[];
  stateOf: (app: AppEntry) => ReturnType<typeof appState>;
}) {
  if (apps.length === 0) return null;
  const info = CATEGORY_INFO[category];
  const Icon = info.icon;
  const inUse = APPS.filter(
    (app) => app.category === category && stateOf(app).tone !== APP_TONE.off,
  ).length;
  const ready = apps.filter((app) => app.available);
  return (
    <section
      aria-labelledby={`shelf-${category}`}
      className="grid gap-4 border-t pt-6 md:grid-cols-[14rem_1fr] md:gap-8"
    >
      <div className="grid content-start gap-2">
        <span className="grid size-9 place-items-center rounded-lg border bg-card text-muted-foreground shadow-xs">
          <Icon className="size-5" />
        </span>
        <h2
          id={`shelf-${category}`}
          className="font-display font-wide text-xl font-semibold tracking-tight"
        >
          {info.name}
        </h2>
        <p className="text-sm text-muted-foreground">{info.description}</p>
        <p className="flex items-center gap-2 text-sm">
          <span
            aria-hidden="true"
            className={cn(
              "size-2 rounded-full",
              inUse > 0 ? "bg-success" : "border border-muted-foreground/50",
            )}
          />
          {inUse > 0 ? `${inUse} in use` : "Nothing set up yet"}
        </p>
      </div>
      <div className="grid content-start gap-4">
        {ready.length > 0 ? (
          <div className="grid gap-4 sm:grid-cols-2">
            {ready.map((app) => (
              <AppCard key={app.slug} app={app} state={stateOf(app)} />
            ))}
          </div>
        ) : null}
        <SoonList apps={apps.filter((app) => !app.available)} />
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

      <div className="grid gap-10">
        {CATEGORIES.map((each) => (
          <CategoryShelf
            key={each}
            category={each}
            apps={apps.filter((app) => app.category === each)}
            stateOf={stateOf}
          />
        ))}
      </div>
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
