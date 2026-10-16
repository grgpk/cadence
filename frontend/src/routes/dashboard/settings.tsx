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
import { Textarea } from "../../components/ui/textarea";
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
import { QUESTIONS } from "../../domains/widget/booking-form";
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
  [SETTINGS_SECTIONS.rules]: "Booking rules",
  [SETTINGS_SECTIONS.questions]: "Questions to ask",
  [SETTINGS_SECTIONS.appearance]: "Theme",
  [SETTINGS_SECTIONS.account]: "Account",
} as const;
type SectionId = keyof typeof SECTION_TITLES;
/** The same limits the API enforces. */
const NOTICE_HOURS = { min: 0, max: 168 };
const WINDOW_DAYS = { min: 1, max: 90 };
const BUFFER_MINUTES = { min: 0, max: 120 };

/** A number field's value: empty while the visitor is between two numbers. */
function numberValue(value: number): number | "" {
  return Number.isNaN(value) ? "" : value;
}

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
      // The public page is cached under the old link and the old answers.
      await queryClient.invalidateQueries({
        queryKey: QUERY_KEYS.bookingPage(sent.public_slug),
      });
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
        description="The link you share and what people read when they open it."
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

        <Field id={`${ID}-title`} label="Title">
          <Input
            id={`${ID}-title`}
            required
            maxLength={80}
            value={values.title}
            onChange={(event) => update({ title: event.target.value })}
          />
        </Field>

        <Field id={`${ID}-description`} label="Description" optional>
          <Textarea
            id={`${ID}-description`}
            rows={3}
            maxLength={300}
            value={values.description}
            onChange={(event) => update({ description: event.target.value })}
          />
        </Field>
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

      <Section
        id={SETTINGS_SECTIONS.rules}
        description="How soon and how far ahead people can book, and whether your page takes bookings at all."
      >
        <div className="grid items-start gap-5 sm:grid-cols-3">
          <Field
            id={`${ID}-notice`}
            label="Minimum notice"
            help="Between booking and the call. 0 allows any open time."
          >
            <div className="flex items-center gap-2">
              <Input
                id={`${ID}-notice`}
                type="number"
                inputMode="numeric"
                required
                min={NOTICE_HOURS.min}
                max={NOTICE_HOURS.max}
                value={numberValue(values.minimum_notice_hours)}
                aria-describedby={fieldHelpId(`${ID}-notice`)}
                onChange={(event) =>
                  update({ minimum_notice_hours: event.target.valueAsNumber })
                }
                className="w-24 text-clock text-[0.9375rem]"
              />
              <span className="text-sm text-muted-foreground">hours</span>
            </div>
          </Field>
          <Field
            id={`${ID}-window`}
            label="Booking window"
            help="How far ahead your page shows times."
          >
            <div className="flex items-center gap-2">
              <Input
                id={`${ID}-window`}
                type="number"
                inputMode="numeric"
                required
                min={WINDOW_DAYS.min}
                max={WINDOW_DAYS.max}
                value={numberValue(values.booking_window_days)}
                aria-describedby={fieldHelpId(`${ID}-window`)}
                onChange={(event) =>
                  update({ booking_window_days: event.target.valueAsNumber })
                }
                className="w-24 text-clock text-[0.9375rem]"
              />
              <span className="text-sm text-muted-foreground">days</span>
            </div>
          </Field>
          <Field
            id={`${ID}-buffer`}
            label="Gap between calls"
            help="Kept free before and after each call. 0 allows back to back."
          >
            <div className="flex items-center gap-2">
              <Input
                id={`${ID}-buffer`}
                type="number"
                inputMode="numeric"
                required
                min={BUFFER_MINUTES.min}
                max={BUFFER_MINUTES.max}
                step={5}
                value={numberValue(values.buffer_minutes)}
                aria-describedby={fieldHelpId(`${ID}-buffer`)}
                onChange={(event) =>
                  update({ buffer_minutes: event.target.valueAsNumber })
                }
                className="w-24 text-clock text-[0.9375rem]"
              />
              <span className="text-sm text-muted-foreground">minutes</span>
            </div>
          </Field>
        </div>
        <div className="flex items-center justify-between gap-4">
          <div>
            <Label htmlFor={`${ID}-active`}>Take bookings</Label>
            <p className="mt-1 text-sm text-muted-foreground">
              Turn off to pause your page. Your link then says you are not taking
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
        id={SETTINGS_SECTIONS.questions}
        description="The form always asks for a name, email and phone number. Turn on anything else you want to know before a call."
      >
        <div className="grid gap-4">
          {QUESTIONS.map((question) => (
            <div key={question.flag} className="flex items-center justify-between gap-4">
              <Label htmlFor={`${ID}-${question.flag}`} className="font-normal">
                {question.label}
              </Label>
              <Switch
                id={`${ID}-${question.flag}`}
                checked={values[question.flag]}
                onCheckedChange={(checked) => update({ [question.flag]: checked })}
              />
            </div>
          ))}
        </div>
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
        description="Your booking page, its rules, what the form asks and how the app looks. Calendars and meeting apps are under Apps."
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
