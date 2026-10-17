import { cn } from "cn";
import { useRef, type ReactNode } from "react";

import { Label } from "../../components/ui/label";

/** The id a control passes as `aria-describedby` to be read with its help or error. */
export function fieldHelpId(id: string): string {
  return `${id}-help`;
}

/**
 * A line that opens and closes its own height, so the form below it slides
 * instead of jumping. Closed, it keeps its last words while it shrinks.
 */
function Reveal({ open, children }: { open: boolean; children: ReactNode }) {
  return (
    <div
      className={cn(
        "grid transition-[grid-template-rows,opacity] duration-200 ease-out",
        open ? "grid-rows-[1fr] opacity-100" : "grid-rows-[0fr] opacity-0",
      )}
    >
      <div
        className={cn(
          "min-h-0 overflow-hidden transition-transform duration-200 ease-out",
          open ? "translate-y-0" : "-translate-y-1",
        )}
      >
        {children}
      </div>
    </div>
  );
}

/** What is wrong with a control, sliding in under it and out again once fixed. */
export function FieldError({ id, error }: { id?: string; error?: string | null }) {
  const last = useRef(error);
  if (error) last.current = error;
  return (
    <Reveal open={Boolean(error)}>
      {error ? (
        <p id={id} role="alert" className="pt-1.5 text-sm text-destructive">
          {error}
        </p>
      ) : (
        <p aria-hidden="true" className="pt-1.5 text-sm text-destructive">
          {last.current}
        </p>
      )}
    </Reveal>
  );
}

/** A labelled control with the line under it: help, or the error that replaces it. */
export function Field({
  id,
  label,
  optional = false,
  help,
  error,
  className,
  children,
}: {
  /** The control's id. */
  id: string;
  label: string;
  optional?: boolean;
  help?: ReactNode;
  error?: string | null;
  className?: string;
  children: ReactNode;
}) {
  return (
    <div className={cn("grid", className)}>
      <div className="grid gap-1.5">
        <Label htmlFor={id}>
          {label}
          {optional ? (
            <span className="font-normal text-muted-foreground">(optional)</span>
          ) : null}
        </Label>
        {children}
      </div>
      <FieldError id={error ? fieldHelpId(id) : undefined} error={error} />
      {help ? (
        <Reveal open={!error}>
          <p
            id={error ? undefined : fieldHelpId(id)}
            aria-hidden={error ? true : undefined}
            className="pt-1.5 text-sm text-muted-foreground"
          >
            {help}
          </p>
        </Reveal>
      ) : null}
    </div>
  );
}
