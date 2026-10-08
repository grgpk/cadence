import { cn } from "cn";

import { APP_NAME } from "../../lib/config";

/** Three uneven bars: beats in a rhythm. The middle beat carries the signal color. */
export function LogoMark({ className }: { className?: string }) {
  return (
    <svg
      viewBox="0 0 20 20"
      aria-hidden="true"
      className={cn("size-5 shrink-0", className)}
    >
      <rect x="2" y="7" width="4" height="11" rx="1.5" className="fill-foreground" />
      <rect x="8" y="2" width="4" height="16" rx="1.5" className="fill-signal" />
      <rect x="14" y="10" width="4" height="8" rx="1.5" className="fill-foreground" />
    </svg>
  );
}

export function Logo({ className }: { className?: string }) {
  return (
    <span className={cn("inline-flex items-center gap-2", className)}>
      <LogoMark />
      <span className="font-display text-lg leading-none font-semibold tracking-tight">
        {APP_NAME}
      </span>
    </span>
  );
}
