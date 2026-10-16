import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { createFileRoute, useBlocker } from "@tanstack/react-router";
import { Copy, Loader2 } from "lucide-react";
import { useEffect, useRef, useState, type ReactNode } from "react";
import { toast } from "sonner";

import type { HostSettings } from "../../bindings/HostSettings";
import { Button } from "../../components/ui/button";
import { Input } from "../../components/ui/input";
import { Label } from "../../components/ui/label";
import { Skeleton } from "../../components/ui/skeleton";
import { Switch } from "../../components/ui/switch";
import { AccountSection } from "../../domains/dashboard/account-section";
import { AppearanceSection } from "../../domains/dashboard/appearance-section";
import { copyBookingLink } from "../../domains/dashboard/booking-link";
import { hostSettingsQuery, saveHostSettings } from "../../domains/dashboard/host-api";
import { SETTINGS_SECTIONS } from "../../domains/dashboard/routes";
import { ConfirmDialog } from "../../domains/shared/confirm-dialog";
import { ErrorState } from "../../domains/shared/error-state";
import { Field, fieldHelpId } from "../../domains/shared/field";
import { PageHeader } from "../../domains/shared/page-header";
import { PanelCard } from "../../domains/shared/panel-card";
import {
  ALL_COUNTRY_TIMEZONES,
  detectBrowserCountryTimezone,
} from "../../domains/shared/phone/country";
import { TimezoneCombobox } from "../../domains/widget/timezone-combobox";
import { ApiError } from "../../lib/api";
import { APP_NAME } from "../../lib/config";
import { errorMessage } from "../../lib/error-message";
import { QUERY_KEYS } from "../../lib/query-keys";
import { bookingPath } from "../../lib/routes";

export const Route = createFileRoute("/dashboard/settings")({
  head: () => ({ meta: [{ title: `Settings | ${APP_NAME}` }] }),
  component: SettingsPage,
});

const ID = "settings";

const SECTION_TITLES = {
  [SETTINGS_SECTIONS.bookingPage]: "Booking page",
  [SETTINGS_SECTIONS.time]: "Time",
  [SETTINGS_SECTIONS.appearance]: "Theme",
  [SETTINGS_SECTIONS.account]: "Account",
} as const;
type SectionId = keyof typeof SECTION_TITLES;
/** One group of settings. Other pages link to it by `id`. */
function Section({
  id,
  description,
  children,
}: {
  id: SectionId;
  description: string;
  children: ReactNode;
}) {
  return (
    <PanelCard id={id} title={SECTION_TITLES[id]} description={description}>
      {children}
    </PanelCard>
  );
}

function SettingsForm({ saved }: { saved: HostSettings }) {
  const queryClient = useQueryClient();
  const [values, setValues] = useState(saved);
  const [slugError, setSlugError] = useState<string | null>(null);
  const [confirmingLink, setConfirmingLink] = useState(false);
  const update = (patch: Partial<HostSettings>) => setValues({ ...values, ...patch });

  // Settings can change behind the form (a refetch, another tab). Follow them
  // while nothing here has been edited; local edits are never thrown away.
  const followed = useRef(saved);
  useEffect(() => {
    const before = followed.current;
    followed.current = saved;
    setValues((current) =>
      JSON.stringify(current) === JSON.stringify(before) ? saved : current,
    );
  }, [saved]);

  // The sections only exist once the settings have loaded, after the browser
  // has already given up on the link's anchor.
  useEffect(() => {
    const section = window.location.hash.slice(1);
    if (section) document.getElementById(section)?.scrollIntoView();
  }, []);

  const save = useMutation({
    mutationFn: saveHostSettings,
    onSuccess: async (next, sent) => {
      queryClient.setQueryData(QUERY_KEYS.hostSettings, next);
      // The public link is cached under the old link and the old pause.
      await Promise.all([
        queryClient.invalidateQueries({
          queryKey: QUERY_KEYS.hostPage(sent.public_slug),
        }),
        queryClient.invalidateQueries({
          queryKey: QUERY_KEYS.bookingPage(sent.public_slug),
        }),
      ]);
      setValues(next);
      setSlugError(null);
      toast.success("Changes saved");
    },
    onError: (error) => {
      if (error instanceof ApiError && error.isConflict) {
        setSlugError("Someone else has this link. Try another one.");
        document.getElementById(`${ID}-slug`)?.focus();
        return;
      }
      toast.error(`Changes were not saved. ${errorMessage(error)}`);
    },
  });

  const linkPrefix = `${window.location.host}${bookingPath("")}`;
  const browserTimezone = detectBrowserCountryTimezone();
  const selectedTimezone = ALL_COUNTRY_TIMEZONES.find(
    (timezone) => timezone.iana === values.timezone,
  );
  const unchanged = JSON.stringify(values) === JSON.stringify(saved);

  // Leaving the page would drop the edits without a word.
  useBlocker({
    shouldBlockFn: ({ current, next }) =>
      !unchanged &&
      current.pathname !== next.pathname &&
      !window.confirm("Leave without saving your changes?"),
    enableBeforeUnload: !unchanged,
  });
  const linkChanged = values.public_slug !== saved.public_slug;

  return (
    <form
      className="grid gap-6"
      onSubmit={(event) => {
        event.preventDefault();
        if (linkChanged) setConfirmingLink(true);
        else save.mutate(values);
      }}
    >
      <Section
        id={SETTINGS_SECTIONS.bookingPage}
        description="The link you share. Each kind of call you offer has its own page under it, set up in Event types."
      >
        <Field
          id={`${ID}-slug`}
          label="Link"
          error={slugError}
          help="3 to 40 lowercase letters, numbers and dashes. Changing it breaks links you already shared."
        >
          <div className="flex flex-wrap items-center gap-2">
            <div className="flex min-w-0 flex-1 items-center rounded-md border border-input bg-muted shadow-xs focus-within:border-ring focus-within:ring-[3px] focus-within:ring-ring/50">
              <span className="truncate pl-3 text-sm text-muted-foreground">
                {linkPrefix}
              </span>
              <Input
                id={`${ID}-slug`}
                value={values.public_slug}
                required
                minLength={3}
                maxLength={40}
                pattern="[a-z0-9]+(-[a-z0-9]+)*"
                title="3 to 40 lowercase letters, numbers and single dashes"
                spellCheck={false}
                autoCapitalize="none"
                aria-invalid={Boolean(slugError)}
                aria-describedby={fieldHelpId(`${ID}-slug`)}
                onChange={(event) => {
                  setSlugError(null);
                  update({ public_slug: event.target.value.toLowerCase() });
                }}
                className="min-w-24 flex-1 rounded-l-none border-0 bg-card pl-1.5 shadow-none focus-visible:ring-0"
              />
            </div>
            <Button
              type="button"
              variant="outline"
              onClick={() => copyBookingLink(saved.public_slug)}
            >
              <Copy />
              Copy link
            </Button>
          </div>
        </Field>

        <div className="flex items-center justify-between gap-4">
          <div>
            <Label htmlFor={`${ID}-active`}>Take bookings</Label>
            <p className="mt-1 text-sm text-muted-foreground">
              Turn off to pause your whole page. Your link then says you are not taking
              bookings, and calls already booked stay.
            </p>
          </div>
          <Switch
            id={`${ID}-active`}
            checked={values.is_active}
            onCheckedChange={(checked) => update({ is_active: checked })}
          />
        </div>
      </Section>

      <Section
        id={SETTINGS_SECTIONS.time}
        description="Your weekly hours are read in this timezone. Visitors see times in their own."
      >
        <Field id={`${ID}-timezone`} label="Timezone">
          <div className="flex flex-wrap items-center gap-x-4 gap-y-2">
            <TimezoneCombobox
              id={`${ID}-timezone`}
              selected={selectedTimezone}
              fallbackLabel={values.timezone}
              onSelect={(timezone) => update({ timezone: timezone.iana })}
            />
            {browserTimezone && browserTimezone.iana !== values.timezone ? (
              <Button
                type="button"
                variant="link"
                size="sm"
                className="h-auto p-0"
                onClick={() => update({ timezone: browserTimezone.iana })}
              >
                Use this device's timezone
              </Button>
            ) : null}
          </div>
        </Field>
      </Section>

      {unchanged ? null : (
        <div className="sticky bottom-4 flex flex-wrap items-center gap-2 rounded-xl border bg-card py-2.5 pr-2.5 pl-4 shadow-lg motion-safe:animate-rise">
          <p className="mr-auto text-sm font-medium">Unsaved changes</p>
          <Button
            type="button"
            variant="ghost"
            onClick={() => {
              setValues(saved);
              setSlugError(null);
            }}
          >
            Discard
          </Button>
          <Button type="submit" disabled={save.isPending}>
            {save.isPending ? <Loader2 className="animate-spin" /> : null}
            {save.isPending ? "Saving" : "Save changes"}
          </Button>
        </div>
      )}

      <ConfirmDialog
        open={confirmingLink}
        onOpenChange={setConfirmingLink}
        title="Change your booking link?"
        description={`${linkPrefix}${saved.public_slug} stops working as soon as you save. Anyone you already sent it to will need the new link.`}
        confirmLabel="Save changes"
        onConfirm={() => save.mutate(values)}
      />
    </form>
  );
}

function SettingsPage() {
  const { user } = Route.useRouteContext();
  const query = useQuery(hostSettingsQuery);

  return (
    <>
      <PageHeader
        title="Settings"
        description="Your link, timezone and how the app looks. The calls you offer are under Event types; calendars and meeting apps are under Apps."
      />
      {query.isPending ? (
        <div className="grid gap-6">
          <Skeleton className="h-80 w-full rounded-xl" />
          <Skeleton className="h-32 w-full rounded-xl" />
          <Skeleton className="h-40 w-full rounded-xl" />
          <Skeleton className="h-64 w-full rounded-xl" />
        </div>
      ) : query.isError ? (
        <ErrorState
          title="Your settings did not load"
          error={query.error}
          onRetry={() => void query.refetch()}
        />
      ) : (
        <div className="grid items-start gap-10 xl:grid-cols-[minmax(0,42rem)_minmax(0,1fr)]">
          <div className="grid max-w-2xl gap-6">
            <SettingsForm saved={query.data} />
            <Section
              id={SETTINGS_SECTIONS.appearance}
              description="How the app looks on this device. Saves on its own."
            >
              <AppearanceSection />
            </Section>
            {user ? (
              <Section
                id={SETTINGS_SECTIONS.account}
                description="Your own name and password. These save on their own."
              >
                <AccountSection user={user} />
              </Section>
            ) : null}
          </div>
          <nav
            aria-label="Settings sections"
            className="sticky top-8 hidden gap-1 border-l pl-4 text-sm xl:grid"
          >
            {Object.entries(SECTION_TITLES).map(([id, title]) => (
              <a
                key={id}
                href={`#${id}`}
                className="rounded-sm py-1 text-muted-foreground transition-colors outline-none hover:text-foreground focus-visible:ring-[3px] focus-visible:ring-ring/50"
              >
                {title}
              </a>
            ))}
          </nav>
        </div>
      )}
    </>
  );
}
