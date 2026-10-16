import type { ComingSoonApp } from "./catalog";

/** Apps that are on the way, kept quiet under what already works. */
export function ComingSoon({ apps }: { apps: readonly ComingSoonApp[] }) {
  return (
    <div className="flex flex-wrap items-center gap-x-3 gap-y-2 text-sm">
      <span className="text-muted-foreground">Coming soon</span>
      <ul className="flex flex-wrap gap-2">
        {apps.map((app) => {
          const Icon = app.icon;
          return (
            <li
              key={app.name}
              title={app.description}
              className="inline-flex items-center gap-1.5 rounded-full border border-dashed bg-card px-2.5 py-1 text-xs font-medium text-muted-foreground"
            >
              <Icon className="size-3.5" />
              {app.name}
            </li>
          );
        })}
      </ul>
    </div>
  );
}
