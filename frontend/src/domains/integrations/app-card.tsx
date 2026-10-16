import { Link } from "@tanstack/react-router";
import { cn } from "cn";
import { Check, ChevronRight, Link as LinkIcon, Webhook } from "lucide-react";

import { Badge } from "../../components/ui/badge";
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

/** A booked call and the link people join it from. */
function ConferencingSketch() {
  return (
    <div aria-hidden="true" className="flex h-9 items-center gap-2">
      <span className="flex h-full shrink-0 flex-col justify-center rounded-sm border-l-4 border-noon bg-primary px-2 text-[0.6875rem] leading-tight text-primary-foreground">
        <span className="font-semibold">Ana</span>
        <span className="text-clock opacity-80">10:00</span>
      </span>
      <span className="h-px flex-1 border-t border-dashed border-foreground/40" />
      <span className="flex h-7 min-w-0 items-center gap-1.5 rounded-full border bg-card px-2.5 text-xs text-muted-foreground">
        <LinkIcon className="size-3 shrink-0" />
        <span className="truncate">meet.google.com/ana-call</span>
      </span>
    </div>
  );
}

/** A booked call, posted on to the host's own server. */
function AutomationSketch() {
  return (
    <div aria-hidden="true" className="flex h-9 items-center gap-2">
      <span className="flex h-full shrink-0 flex-col justify-center rounded-sm border-l-4 border-noon bg-primary px-2 text-[0.6875rem] leading-tight text-primary-foreground">
        <span className="font-semibold">Ana</span>
        <span className="text-clock opacity-80">10:00</span>
      </span>
      <span className="h-px flex-1 border-t border-dashed border-foreground/40" />
      <span className="flex h-7 min-w-0 items-center gap-1.5 rounded-full border bg-card px-2.5 font-mono text-xs text-muted-foreground">
        <Webhook className="size-3 shrink-0" />
        <span className="truncate">POST /hooks/cadence</span>
      </span>
    </div>
  );
}

const SKETCHES = {
  [APP_CATEGORY.calendar]: CalendarSketch,
  [APP_CATEGORY.conferencing]: ConferencingSketch,
  [APP_CATEGORY.automation]: AutomationSketch,
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
        "grid gap-4 rounded-xl border bg-muted/40 p-4 text-left transition-colors hover:border-foreground/30",
        selected && "border-foreground bg-card hover:border-foreground",
        FOCUS,
      )}
    >
      <div className="rounded-lg border bg-background p-3">
        <Sketch />
      </div>
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
