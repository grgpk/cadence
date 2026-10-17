import { createFileRoute } from "@tanstack/react-router";

import { CalendlyImport } from "../../../domains/import/calendly-import";
import { PageHeader } from "../../../domains/shared/page-header";
import { APP_NAME } from "../../../lib/config";

export const Route = createFileRoute("/dashboard/event-types/import")({
  head: () => ({ meta: [{ title: `Import from Calendly | ${APP_NAME}` }] }),
  component: CalendlyImportPage,
});

function CalendlyImportPage() {
  return (
    <>
      <PageHeader
        title="Import from Calendly"
        description="Bring your event types, weekly hours and timezone over in a minute, then share your Cadence link."
      />
      <CalendlyImport />
    </>
  );
}
