import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { createFileRoute } from "@tanstack/react-router";
import { Copy, Loader2 } from "lucide-react";
import { useEffect, useState, type ReactNode } from "react";
import { toast } from "sonner";

import type { HostSettings } from "../../bindings/HostSettings";
import { Button } from "../../components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "../../components/ui/card";
import { Input } from "../../components/ui/input";
import { Label } from "../../components/ui/label";
import { Skeleton } from "../../components/ui/skeleton";
import { Switch } from "../../components/ui/switch";
import { Textarea } from "../../components/ui/textarea";
import { copyBookingLink } from "../../domains/dashboard/booking-link";
import { GoogleCalendarConnection } from "../../domains/dashboard/GoogleCalendarConnection";
import { hostSettingsQuery, saveHostSettings } from "../../domains/dashboard/host-api";
import { SETTINGS_SECTIONS } from "../../domains/dashboard/routes";
import { ConfirmDialog } from "../../domains/shared/ConfirmDialog";
import { ErrorState } from "../../domains/shared/ErrorState";
import { Field, fieldHelpId } from "../../domains/shared/Field";
import { PageHeader } from "../../domains/shared/PageHeader";
import {
  ALL_COUNTRY_TIMEZONES,
  detectBrowserCountryTimezone,
} from "../../domains/shared/phone/country";
import { QUESTIONS } from "../../domains/widget/booking-form";
import { TimezoneCombobox } from "../../domains/widget/TimezoneCombobox";
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
const SECURE_LINK_PREFIX = "https://";

/** One group of settings. Other pages link to it by `id`. */
function Section({
  id,
  title,
  description,
  children,
}: {
  id: string;
  title: string;
  description: string;
  children: ReactNode;
}) {
  return (
    <Card id={id} className="scroll-mt-16">
      <CardHeader>
        <CardTitle>{title}</CardTitle>
        <CardDescription>{description}</CardDescription>
      </CardHeader>
      <CardContent className="grid gap-5">{children}</CardContent>
    </Card>
  );
}

function SettingsForm({ saved }: { saved: HostSettings }) {
  const queryClient = useQueryClient();
  const [values, setValues] = useState(saved);
  const [slugError, setSlugError] = useState<string | null>(null);
  const [confirmingLink, setConfirmingLink] = useState(false);
  const update = (patch: Partial<HostSettings>) => setValues({ ...values, ...patch });

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
  const linkChanged = values.public_slug !== saved.public_slug;
  const meetingLink = values.meeting_link?.trim() ?? "";
  const meetingLinkInvalid =
    meetingLink !== "" && !meetingLink.startsWith(SECURE_LINK_PREFIX);

  return (
    <form
      className="grid max-w-2xl gap-6"
      onSubmit={(event) => {
        event.preventDefault();
        if (linkChanged) setConfirmingLink(true);
        else save.mutate(values);
      }}
    >
      <Section
        id={SETTINGS_SECTIONS.bookingPage}
        title="Booking page"
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
        title="Time"
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
        id={SETTINGS_SECTIONS.calls}
        title="Where calls happen"
        description="People who book you are sent a link to join. Connect Google Calendar for a new Google Meet link per call, or give one link of your own."
      >
        <GoogleCalendarConnection slug={saved.public_slug} />

        <div className="flex items-center gap-3 text-sm text-muted-foreground">
          <span aria-hidden="true" className="h-px flex-1 bg-border" />
          or
          <span aria-hidden="true" className="h-px flex-1 bg-border" />
        </div>

        <Field
          id={`${ID}-meeting-link`}
          label="Meeting link"
          optional
          error={meetingLinkInvalid ? "Use a link that starts with https://" : null}
          help="Such as a Zoom or Meet room. Used when Google Calendar is not connected."
        >
          <Input
            id={`${ID}-meeting-link`}
            type="url"
            inputMode="url"
            maxLength={500}
            placeholder="https://zoom.us/j/123456789"
            value={values.meeting_link ?? ""}
            aria-invalid={meetingLinkInvalid}
            aria-describedby={fieldHelpId(`${ID}-meeting-link`)}
            onChange={(event) => update({ meeting_link: event.target.value || null })}
          />
        </Field>
      </Section>

      <Section
        id={SETTINGS_SECTIONS.questions}
        title="Questions to ask"
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
          <Button type="button" variant="ghost" onClick={() => setValues(saved)}>
            Discard
          </Button>
          <Button type="submit" disabled={save.isPending || meetingLinkInvalid}>
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
  const query = useQuery(hostSettingsQuery);

  return (
    <>
      <PageHeader
        title="Settings"
        description="Your booking page, where calls happen and what the form asks."
      />
      {query.isPending ? (
        <div className="grid max-w-2xl gap-6">
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
        <SettingsForm saved={query.data} />
      )}
    </>
  );
}
