import type { ComponentProps } from "react";

import { bookingPath } from "../../lib/routes";
import { CopyButton } from "../shared/copy-button";

/** Copies the booking page at `slug` (a host, or a host and a kind of call). */
export function CopyBookingLink({
  slug,
  children = "Copy link",
  ...props
}: Omit<ComponentProps<typeof CopyButton>, "text" | "failure"> & { slug: string }) {
  return (
    <CopyButton
      text={`${window.location.origin}${bookingPath(slug)}`}
      failure="The link could not be copied. Copy it from the address bar."
      {...props}
    >
      {children}
    </CopyButton>
  );
}
