import { Link } from "@tanstack/react-router";
import { Import, X } from "lucide-react";
import { useState } from "react";

import { Button } from "../../components/ui/button";
import { Card } from "../../components/ui/card";
import { ROUTES } from "../../lib/routes";

const DISMISSED_KEY = "cadence-switching-card-dismissed";

function wasDismissed(): boolean {
  try {
    return localStorage.getItem(DISMISSED_KEY) === "yes";
  } catch {
    return false;
  }
}

/**
 * For a new host: their Calendly setup is a minute away. Shown until they
 * have a booking or say not now; the "not now" is kept in this browser.
 */
export function SwitchingCard() {
  const [dismissed, setDismissed] = useState(wasDismissed);
  if (dismissed) return null;
  const dismiss = () => {
    setDismissed(true);
    try {
      localStorage.setItem(DISMISSED_KEY, "yes");
    } catch {
      // Storage can be blocked; the card stays hidden for this visit.
    }
  };
  return (
    <Card className="flex-row flex-wrap items-center gap-4 px-5 py-4">
      <Import className="size-5 shrink-0 text-dusk" aria-hidden />
      <div className="min-w-0 flex-1 basis-64">
        <h2 className="font-semibold">Coming from Calendly?</h2>
        <p className="text-sm text-muted-foreground">
          Bring your event types, weekly hours and timezone over in a minute.
        </p>
      </div>
      <div className="flex items-center gap-1">
        <Button asChild>
          <Link to={ROUTES.dashboardImportCalendly}>Import from Calendly</Link>
        </Button>
        <Button variant="ghost" onClick={dismiss}>
          <X />
          Not now
        </Button>
      </div>
    </Card>
  );
}
