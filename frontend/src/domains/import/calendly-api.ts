import type { CalendlyImport } from "../../bindings/CalendlyImport";
import type { CalendlyImported } from "../../bindings/CalendlyImported";
import type { CalendlyPreview } from "../../bindings/CalendlyPreview";
import { api } from "../../lib/api";

/** Reads the Calendly account behind `token`. The token is not kept. */
export function previewCalendly(token: string): Promise<CalendlyPreview> {
  return api<CalendlyPreview>("/api/host/import/calendly/preview", {
    method: "POST",
    body: JSON.stringify({ token }),
  });
}

export function importCalendly(body: CalendlyImport): Promise<CalendlyImported> {
  return api<CalendlyImported>("/api/host/import/calendly", {
    method: "POST",
    body: JSON.stringify(body),
  });
}

/** Where Calendly makes a personal access token. */
export const CALENDLY_TOKEN_URL = "https://calendly.com/integrations/api_webhooks";
