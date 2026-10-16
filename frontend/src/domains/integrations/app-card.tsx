import { Link } from "@tanstack/react-router";
import { cn } from "cn";
import { Check, ChevronRight } from "lucide-react";

import { Badge } from "../../components/ui/badge";
import { ROUTES } from "../../lib/routes";
import { APP_CATEGORY, type AppCategory, type AppEntry, CATEGORY_INFO } from "./catalog";

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

/** One app in the store. The whole card opens its page. */
export function AppCard({ app, installed }: { app: AppEntry; installed: boolean }) {
  return (
    <Link
      to={ROUTES.dashboardApp}
      params={{ slug: app.slug }}
      className={cn(
        "group flex flex-col rounded-xl border bg-card text-card-foreground shadow-xs transition-colors hover:border-foreground/30",
        FOCUS,
      )}
    >
      <div className="flex flex-1 flex-col p-5">
        <div className="flex items-start justify-between gap-3">
          <AppLogo app={app} />
          <AppStatus app={app} installed={installed} />
        </div>
        <h3 className="mt-5 font-display font-wide text-lg font-semibold tracking-tight">
          {app.name}
        </h3>
        <p className="mt-1.5 line-clamp-3 text-sm leading-relaxed text-muted-foreground">
          {app.summary}
        </p>
      </div>
      <div className="flex items-center justify-between border-t px-5 py-3 text-sm">
        <span className="text-muted-foreground">{CATEGORY_INFO[app.category].name}</span>
        <span className="inline-flex items-center gap-0.5 font-medium">
          {installed ? "Manage" : "Set up"}
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

/** A host's day as hour cells: busy ones from the calendar, kept off the page. */
function CalendarSketch() {
  const cells = ["open", "busy", "open", "open", "busy", "busy", "open", "open"] as const;
  return (
    <div aria-hidden="true" className="flex gap-1">
      {cells.map((cell, index) => (
        <span
          key={index}
          className={cn(
            "h-9 flex-1 rounded-sm border",
            cell === "busy"
              ? "border-dusk/50 bg-[repeating-linear-gradient(135deg,var(--color-dusk)_0_1.5px,transparent_1.5px_6px)] opacity-60"
              : index < 4
                ? "border-dawn bg-dawn/35 dark:bg-dawn/10"
                : "border-noon bg-noon/30 dark:bg-noon/10",
          )}
        />
      ))}
    </div>
  );
}

/** A booked call turning into the link people join it from. */
function ConferencingSketch() {
  return (
    <div aria-hidden="true" className="flex h-9 items-center gap-2">
      <span className="flex h-full w-1/4 items-center rounded-sm bg-primary px-2 text-xs font-semibold text-primary-foreground">
        Ana
      </span>
      <span className="h-px flex-1 border-t border-dashed border-foreground/40" />
      <span className="flex h-7 items-center rounded-full border bg-card px-2.5 text-xs text-muted-foreground">
        meet.link/ana
      </span>
    </div>
  );
}

const SKETCHES = {
  [APP_CATEGORY.calendar]: CalendarSketch,
  [APP_CATEGORY.conferencing]: ConferencingSketch,
};

/** A category of apps, picked to show only its apps. */
export function CategoryTile({
  category,
  count,
  selected,
  onToggle,
}: {
  category: AppCategory;
  count: number;
  selected: boolean;
  onToggle: () => void;
}) {
  const info = CATEGORY_INFO[category];
  const Sketch = SKETCHES[category];
  return (
    <button
      type="button"
      aria-pressed={selected}
      onClick={onToggle}
      className={cn(
        "grid gap-5 rounded-xl border bg-muted/40 p-5 text-left transition-colors hover:border-foreground/30",
        selected && "border-foreground bg-card hover:border-foreground",
        FOCUS,
      )}
    >
      <Sketch />
      <span className="flex items-end justify-between gap-3">
        <span>
          <span className="block font-display font-wide text-lg font-semibold tracking-tight">
            {info.name}
          </span>
          <span className="mt-0.5 block text-sm text-muted-foreground">
            {info.description}
          </span>
        </span>
        <span className="text-clock shrink-0 text-sm text-muted-foreground">
          {selected ? "Showing" : `${count} apps`}
        </span>
      </span>
    </button>
  );
}
