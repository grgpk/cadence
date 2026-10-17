import { useMutation, useQueryClient } from "@tanstack/react-query";
import { Link, useNavigate } from "@tanstack/react-router";
import { ExternalLink, Loader2 } from "lucide-react";
import { useId, useState } from "react";
import { toast } from "sonner";

import type { CalendlyPreview } from "../../bindings/CalendlyPreview";
import { Button } from "../../components/ui/button";
import { Checkbox } from "../../components/ui/checkbox";
import { Input } from "../../components/ui/input";
import { Label } from "../../components/ui/label";
import { useClockPreference } from "../../hooks/use-clock-preference";
import { errorMessage } from "../../lib/error-message";
import {
  formatClock,
  formatDate,
  lengthLabel,
  weekdayName,
  zoneName,
} from "../../lib/format";
import { QUERY_KEYS } from "../../lib/query-keys";
import { ROUTES } from "../../lib/routes";
import { PLAN_NAMES } from "../pricing/pricing";
import { PanelCard } from "../shared/panel-card";
import { CALENDLY_TOKEN_URL, importCalendly, previewCalendly } from "./calendly-api";

const PREVIEW_ERRORS = {
  400: "Calendly did not accept that token. Check it can read users, event types and availability.",
  429: "Too many tries. Wait an hour and try again.",
  503: "Calendly did not answer. Try again in a minute.",
};

/** Step one: the token, used once to read the account. */
function TokenForm({ onPreview }: { onPreview: (preview: CalendlyPreview) => void }) {
  const id = useId();
  const [token, setToken] = useState("");
  const preview = useMutation({ mutationFn: previewCalendly, onSuccess: onPreview });
  return (
    <PanelCard
      title="Connect your Calendly account"
      description="Cadence reads your event types, weekly hours and timezone once. Your token is not saved."
    >
      <ol className="grid list-decimal gap-1.5 pl-5 text-sm text-muted-foreground">
        <li>
          In Calendly, open{" "}
          <a
            href={CALENDLY_TOKEN_URL}
            target="_blank"
            rel="noreferrer"
            className="font-medium text-foreground underline underline-offset-4"
          >
            Integrations, API and webhooks
            <ExternalLink className="ml-1 inline size-3.5" aria-hidden />
          </a>
          .
        </li>
        <li>
          Make a personal access token that can read users, event types and availability.
        </li>
        <li>Paste it here.</li>
      </ol>
      <form
        className="grid gap-3 sm:grid-cols-[1fr_auto] sm:items-end"
        onSubmit={(event) => {
          event.preventDefault();
          preview.mutate(token);
        }}
      >
        <div className="grid gap-2">
          <Label htmlFor={`${id}-token`}>Personal access token</Label>
          <Input
            id={`${id}-token`}
            value={token}
            onChange={(event) => setToken(event.target.value)}
            autoComplete="off"
            spellCheck={false}
            required
          />
        </div>
        <Button disabled={preview.isPending || token.trim() === ""}>
          {preview.isPending ? <Loader2 className="animate-spin" /> : null}
          {preview.isPending ? "Reading Calendly" : "Read my Calendly"}
        </Button>
      </form>
      {preview.isError ? (
        <p role="alert" className="text-sm text-destructive">
          {errorMessage(preview.error, PREVIEW_ERRORS)}
        </p>
      ) : null}
    </PanelCard>
  );
}

/** Step two: what would come in, for the host to pick from. */
function Review({
  preview,
  onRestart,
}: {
  preview: CalendlyPreview;
  onRestart: () => void;
}) {
  const id = useId();
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const [hour12] = useClockPreference();
  const [picked, setPicked] = useState(() =>
    preview.event_types.map((kind) => kind.slug),
  );
  const [useTimezone, setUseTimezone] = useState(preview.timezone !== null);
  const [useHours, setUseHours] = useState(preview.weekly.length > 0);
  const allowed = preview.types_allowed;

  const save = useMutation({
    mutationFn: () =>
      importCalendly({
        timezone: useTimezone ? preview.timezone : null,
        event_types: preview.event_types.filter((kind) => picked.includes(kind.slug)),
        weekly: useHours ? preview.weekly : [],
        dates: useHours ? preview.dates : [],
      }),
    onSuccess: async (saved) => {
      await Promise.all(
        [
          QUERY_KEYS.eventTypes,
          QUERY_KEYS.availability,
          QUERY_KEYS.availabilityOverrides,
          QUERY_KEYS.hostSettings,
        ].map((queryKey) => queryClient.invalidateQueries({ queryKey })),
      );
      toast.success(
        saved.event_types_left_out > 0
          ? `Brought in from Calendly. ${saved.event_types_left_out} more ${saved.event_types_left_out === 1 ? "call needs" : "calls need"} ${PLAN_NAMES.pro}.`
          : "Brought in from Calendly.",
      );
      void navigate({ to: ROUTES.dashboardEventTypes });
    },
    onError: (error) => toast.error(`Nothing was brought in. ${errorMessage(error)}`),
  });

  const toggle = (slug: string, on: boolean) =>
    setPicked((current) =>
      on ? [...current, slug] : current.filter((item) => item !== slug),
    );

  return (
    <div className="grid gap-6">
      <PanelCard
        title="Kinds of call"
        description={
          allowed !== null && preview.event_types.length > allowed
            ? `Your plan brings in ${allowed}. ${PLAN_NAMES.pro} brings all ${preview.event_types.length}. The first one fills your current call, so its link keeps working.`
            : "The first one fills your current call, so its link keeps working."
        }
      >
        {preview.event_types.length === 0 ? (
          <p className="text-sm text-muted-foreground">No event types to bring in.</p>
        ) : (
          <ul className="grid gap-3">
            {preview.event_types.map((kind) => (
              <li key={kind.slug} className="flex items-start gap-3">
                <Checkbox
                  id={`${id}-${kind.slug}`}
                  checked={picked.includes(kind.slug)}
                  onCheckedChange={(on) => toggle(kind.slug, on === true)}
                  className="mt-0.5"
                />
                <Label
                  htmlFor={`${id}-${kind.slug}`}
                  className="grid gap-0.5 font-normal"
                >
                  <span className="font-medium">{kind.title}</span>
                  <span className="text-muted-foreground">
                    {lengthLabel(kind.duration_minutes)}, /{kind.slug}
                  </span>
                </Label>
              </li>
            ))}
          </ul>
        )}
        {preview.skipped.length > 0 ? (
          <ul className="grid gap-1 text-sm text-muted-foreground">
            {preview.skipped.map((kind) => (
              <li key={kind.name}>
                {kind.name}: {kind.reason}
              </li>
            ))}
          </ul>
        ) : null}
      </PanelCard>

      <PanelCard
        title="Weekly hours"
        description="These replace the weekly hours you have now."
      >
        {preview.weekly.length === 0 ? (
          <p className="text-sm text-muted-foreground">No weekly hours in Calendly.</p>
        ) : (
          <>
            <ul className="grid gap-1 text-sm">
              {preview.weekly.map((hours) => (
                <li
                  key={`${hours.weekday}-${hours.start_time}`}
                  className="flex justify-between gap-4"
                >
                  <span>{weekdayName(hours.weekday)}</span>
                  <span className="text-clock">
                    {formatClock(hours.start_time, hour12)} to{" "}
                    {formatClock(hours.end_time, hour12)}
                  </span>
                </li>
              ))}
            </ul>
            {preview.dates.length > 0 ? (
              <p className="text-sm text-muted-foreground">
                Also{" "}
                {preview.dates.length === 1 ? "1 date" : `${preview.dates.length} dates`}{" "}
                with their own hours, from {formatDate(preview.dates[0]?.date ?? "")}.
              </p>
            ) : null}
            <div className="flex items-center gap-3">
              <Checkbox
                id={`${id}-hours`}
                checked={useHours}
                onCheckedChange={(on) => setUseHours(on === true)}
              />
              <Label htmlFor={`${id}-hours`} className="font-normal">
                Bring in these hours
              </Label>
            </div>
          </>
        )}
        {preview.timezone ? (
          <div className="flex items-center gap-3">
            <Checkbox
              id={`${id}-timezone`}
              checked={useTimezone}
              onCheckedChange={(on) => setUseTimezone(on === true)}
            />
            <Label htmlFor={`${id}-timezone`} className="font-normal">
              Use {zoneName(preview.timezone)} as my timezone
            </Label>
          </div>
        ) : null}
      </PanelCard>

      <div className="flex flex-wrap justify-end gap-2">
        <Button variant="ghost" onClick={onRestart} disabled={save.isPending}>
          Use another token
        </Button>
        <Button
          onClick={() => save.mutate()}
          disabled={save.isPending || (picked.length === 0 && !useHours && !useTimezone)}
        >
          {save.isPending ? <Loader2 className="animate-spin" /> : null}
          {save.isPending ? "Bringing in" : "Bring them in"}
        </Button>
      </div>
      <p className="text-sm text-muted-foreground">
        Nothing changes until you choose Bring them in. You can still edit everything
        afterwards in{" "}
        <Link to={ROUTES.dashboardEventTypes} className="underline underline-offset-4">
          Event types
        </Link>{" "}
        and{" "}
        <Link to={ROUTES.dashboardAvailability} className="underline underline-offset-4">
          Availability
        </Link>
        .
      </p>
    </div>
  );
}

/** Brings a host's Calendly setup into Cadence: token, review, save. */
export function CalendlyImport() {
  const [preview, setPreview] = useState<CalendlyPreview | null>(null);
  return preview ? (
    <Review preview={preview} onRestart={() => setPreview(null)} />
  ) : (
    <TokenForm onPreview={setPreview} />
  );
}
