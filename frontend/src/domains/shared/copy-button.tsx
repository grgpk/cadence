import { Check, Copy } from "lucide-react";
import { type ComponentProps, useEffect, useRef, useState } from "react";
import { toast } from "sonner";

import { Button } from "../../components/ui/button";

/** How long the button shows its checkmark after a copy. */
const COPIED_MS = 1500;

/**
 * Copies `text` and swaps its icon for a checkmark for a moment. Only a copy
 * that fails says so, with `failure`.
 */
export function CopyButton({
  text,
  failure,
  children,
  ...props
}: Omit<ComponentProps<typeof Button>, "onClick" | "asChild"> & {
  text: string;
  failure: string;
}) {
  const [copied, setCopied] = useState(false);
  const timer = useRef<number | undefined>(undefined);
  useEffect(() => () => window.clearTimeout(timer.current), []);

  function copy() {
    navigator.clipboard.writeText(text).then(
      () => {
        setCopied(true);
        window.clearTimeout(timer.current);
        timer.current = window.setTimeout(() => setCopied(false), COPIED_MS);
      },
      () => toast.error(failure),
    );
  }

  return (
    <Button type="button" {...props} onClick={copy}>
      {copied ? (
        <Check className="text-success motion-safe:animate-in motion-safe:zoom-in-50 motion-safe:duration-200" />
      ) : (
        <Copy />
      )}
      {children}
      <span className="sr-only" aria-live="polite">
        {copied ? "Copied" : ""}
      </span>
    </Button>
  );
}
