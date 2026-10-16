import { cn } from "cn";

import { APP_NAME } from "../../lib/config";

/** A "C" that is also a clock dial. The signal hand points at "now" where the C opens. */
export function LogoMark({ className }: { className?: string }) {
  return (
    <svg
      viewBox="0 0 20 20"
      aria-hidden="true"
      fill="none"
      strokeLinecap="round"
      className={cn("size-5 shrink-0", className)}
    >
      <path
        d="M15.3 4.7A7.5 7.5 0 1 0 15.3 15.3"
        strokeWidth="2.5"
        className="stroke-foreground"
      />
      <path d="M10 10V6" strokeWidth="2" className="stroke-foreground" />
      <path d="M10 10H16" strokeWidth="2" className="stroke-signal" />
      <circle cx="17.25" cy="10" r="1.75" className="fill-signal" />
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
