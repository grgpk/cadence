import { Link } from "@tanstack/react-router";
import { cn } from "cn";
import { Check, ChevronRight, LayoutGrid } from "lucide-react";

import { Badge } from "../../components/ui/badge";
import { Button } from "../../components/ui/button";
import { ROUTES } from "../../lib/routes";
import {
  APP_CATEGORY,
  APP_TONE,
  type AppCategory,
  type AppEntry,
  type AppTone,
  CATEGORY_INFO,
} from "./catalog";

const LOGO_SIZES = {
  sm: { box: "size-9 rounded-lg", icon: "size-5" },
  md: { box: "size-12 rounded-xl", icon: "size-7" },
  lg: { box: "size-16 rounded-2xl", icon: "size-9" },
};

/** An app's mark on a tile, the same size whatever the icon. */
export function AppLogo({
  app,
  size = "md",
}: {
  app: AppEntry;
  size?: keyof typeof LOGO_SIZES;
}) {
  const Icon = app.icon;
  return (
    <span
      className={cn(
        "grid shrink-0 place-items-center border bg-card shadow-xs",
        LOGO_SIZES[size].box,
      )}
    >
      <Icon className={cn(LOGO_SIZES[size].icon, "text-muted-foreground")} />
    </span>
  );
}

/** Whether the host uses an app, or that it is not out yet. */
export function AppStatus({ app, installed }: { app: AppEntry; installed: boolean }) {
  if (!app.available) return <Badge variant="outline">Coming soon</Badge>;
  if (!installed) return null;
  return (
    <Badge variant="secondary" className="text-success">
      <Check />
      In use
    </Badge>
  );
}

const FOCUS = "outline-none focus-visible:ring-[3px] focus-visible:ring-ring/50";

const TONE_DOT: Record<AppTone, string> = {
  [APP_TONE.on]: "bg-success",
  [APP_TONE.warn]: "bg-warning",
  [APP_TONE.off]: "border border-muted-foreground/50",
};

/** One app in the store, with what it is doing for the host. The whole card opens its page. */
export function AppCard({
  app,
  state,
}: {
  app: AppEntry;
  state: { tone: AppTone; text: string };
}) {
  return (
    <Link
      to={ROUTES.dashboardApp}
      params={{ slug: app.slug }}
      className={cn(
        "flex flex-col rounded-xl border bg-card text-card-foreground shadow-xs transition-colors hover:border-foreground/30",
        FOCUS,
      )}
    >
      <div className="flex flex-1 flex-col gap-4 p-5">
        <div className="flex items-center gap-3">
          <AppLogo app={app} />
          <div className="min-w-0">
            <h3 className="truncate font-display font-wide text-lg font-semibold tracking-tight">
              {app.name}
            </h3>
            <p className="text-sm text-muted-foreground">
              {CATEGORY_INFO[app.category].name}
            </p>
          </div>
        </div>
        <p className="line-clamp-3 text-sm leading-relaxed text-muted-foreground">
          {app.summary}
        </p>
      </div>
      <div className="flex items-center justify-between gap-3 border-t px-5 py-3 text-sm">
        <span
          className={cn(
            "flex min-w-0 items-center gap-2",
            state.tone === APP_TONE.off ? "text-muted-foreground" : "font-medium",
          )}
        >
          <span
            aria-hidden="true"
            className={cn("size-2 shrink-0 rounded-full", TONE_DOT[state.tone])}
          />
          <span className="truncate">{state.text}</span>
        </span>
        <span className="inline-flex shrink-0 items-center gap-0.5 font-medium">
          {state.tone === APP_TONE.off ? "Set up" : "Manage"}
          <ChevronRight className="size-4 text-muted-foreground" />
        </span>
      </div>
    </Link>
  );
}

/** An app that is on the way: listed so hosts know, with nothing to open. */
export function SoonRow({ app }: { app: AppEntry }) {
  return (
    <li className="flex items-center gap-3 rounded-xl border border-dashed px-4 py-3">
      <AppLogo app={app} size="sm" />
      <div className="min-w-0">
        <p className="font-medium">{app.name}</p>
        <p className="truncate text-sm text-muted-foreground">{app.summary}</p>
      </div>
    </li>
  );
}

/** The store's category filter: all apps, or one category at a time. */
export function CategoryFilter({
  selected,
  countOf,
  onSelect,
}: {
  selected: AppCategory | undefined;
  countOf: (category: AppCategory | undefined) => number;
  onSelect: (category: AppCategory | undefined) => void;
}) {
  const options = [undefined, ...Object.values(APP_CATEGORY)];
  return (
    <div className="grid gap-2">
      <fieldset className="flex flex-wrap gap-2">
        <legend className="sr-only">Categories</legend>
        {options.map((category) => {
          const info = category ? CATEGORY_INFO[category] : null;
          const Icon = info?.icon ?? LayoutGrid;
          const active = selected === category;
          return (
            <Button
              key={category ?? "all"}
              type="button"
              size="sm"
              variant={active ? "default" : "outline"}
              aria-pressed={active}
              onClick={() => onSelect(category)}
              className="rounded-full"
            >
              <Icon />
              {info?.name ?? "All apps"}
              <span
                className={cn("text-clock text-xs", !active && "text-muted-foreground")}
              >
                {countOf(category)}
              </span>
            </Button>
          );
        })}
      </fieldset>
      {selected ? (
        <p className="text-sm text-muted-foreground">
          {CATEGORY_INFO[selected].description}
        </p>
      ) : null}
    </div>
  );
}
