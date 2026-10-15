import { cn } from "cn";
import type { ReactNode } from "react";

import { Label } from "../../components/ui/label";

/** The id a control passes as `aria-describedby` to be read with its help or error. */
export function fieldHelpId(id: string): string {
  return `${id}-help`;
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
    <div className={cn("grid gap-1.5", className)}>
      <Label htmlFor={id}>
        {label}
        {optional ? (
          <span className="font-normal text-muted-foreground">(optional)</span>
        ) : null}
      </Label>
      {children}
      {error ? (
        <p id={fieldHelpId(id)} role="alert" className="text-sm text-destructive">
          {error}
        </p>
      ) : help ? (
        <p id={fieldHelpId(id)} className="text-sm text-muted-foreground">
          {help}
        </p>
      ) : null}
    </div>
  );
}
