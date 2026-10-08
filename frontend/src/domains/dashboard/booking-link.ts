import { toast } from "sonner";

import { bookingPath } from "../../lib/routes";

export function copyBookingLink(slug: string) {
  const link = `${window.location.origin}${bookingPath(slug)}`;
  navigator.clipboard.writeText(link).then(
    () => toast.success("Link copied"),
    () => toast.error("The link could not be copied. Copy it from the address bar."),
  );
}
