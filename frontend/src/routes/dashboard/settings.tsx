import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { createFileRoute } from "@tanstack/react-router";
import { Copy } from "lucide-react";
import { useState } from "react";
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
import { GoogleCalendarCard } from "../../domains/dashboard/GoogleCalendarCard";
import { ErrorState } from "../../domains/shared/ErrorState";
import { PageHeader } from "../../domains/shared/PageHeader";
import {
  ALL_COUNTRY_TIMEZONES,
  detectBrowserCountryTimezone,
} from "../../domains/shared/phone/country";
import { QUESTIONS } from "../../domains/widget/booking-form";
import { getHostSettings, saveHostSettings } from "../../domains/widget/cadence-api";
import { TimezoneCombobox } from "../../domains/widget/TimezoneCombobox";
import { ApiError } from "../../lib/api";
import { APP_NAME } from "../../lib/config";
import { QUERY_KEYS } from "../../lib/query-keys";
import { bookingPath } from "../../lib/routes";

export const Route = createFileRoute("/dashboard/settings")({
  head: () => ({ meta: [{ title: `Settings | ${APP_NAME}` }] }),
  component: SettingsPage,
});

const ID = "settings";

function SettingsForm({ saved }: { saved: HostSettings }) {
  const queryClient = useQueryClient();
  const [values, setValues] = useState(saved);
  const [slugError, setSlugError] = useState<string | null>(null);
  const update = (patch: Partial<HostSettings>) => setValues({ ...values, ...patch });

  const save = useMutation({
    mutationFn: saveHostSettings,
    onSuccess: (next) => {
      queryClient.setQueryData(QUERY_KEYS.hostSettings, next);
      setValues(next);
      setSlugError(null);
      toast.success("Changes saved");
    },
    onError: (error) => {
      if (error instanceof ApiError && error.isConflict) {
        setSlugError("Someone else has this link. Try another one.");
        return;
      }
      toast.error(`Changes were not saved. ${error.message}`);
    },
  });

  const linkPrefix = `${window.location.host}${bookingPath("")}`;
  const browserTimezone = detectBrowserCountryTimezone();
  const selectedTimezone = ALL_COUNTRY_TIMEZONES.find(
    (timezone) => timezone.iana === values.timezone,
  );
  const unchanged = JSON.stringify(values) === JSON.stringify(saved);

  function copyLink() {
    const link = `${window.location.origin}${bookingPath(saved.public_slug)}`;
    navigator.clipboard.writeText(link).then(
      () => toast.success("Link copied"),
      () => toast.error("The link could not be copied. Copy it from the field."),
    );
  }

  return (
    <form
      className="grid gap-6"
      onSubmit={(event) => {
        event.preventDefault();
        save.mutate(values);
      }}
    >
      <Card>
        <CardHeader>
          <CardTitle>Your link</CardTitle>
          <CardDescription>
            Anyone with this link can book a call in the hours you set.
          </CardDescription>
        </CardHeader>
        <CardContent className="grid gap-5">
          <div className="grid gap-1.5">
            <Label htmlFor={`${ID}-slug`}>Link</Label>
            <div className="flex flex-wrap items-center gap-2">
              <div className="flex min-w-0 flex-1 items-center rounded-md border border-input bg-muted shadow-xs focus-within:border-ring focus-within:ring-[3px] focus-within:ring-ring/50">
                <span className="truncate pl-3 text-sm text-muted-foreground">
                  {linkPrefix}
                </span>
                <Input
                  id={`${ID}-slug`}
                  value={values.public_slug}
                  maxLength={40}
                  spellCheck={false}
                  autoCapitalize="none"
                  aria-invalid={Boolean(slugError)}
                  aria-describedby={`${ID}-slug-help`}
                  onChange={(event) => {
                    setSlugError(null);
                    update({ public_slug: event.target.value.toLowerCase() });
                  }}
                  className="min-w-24 flex-1 rounded-l-none border-0 bg-card pl-1.5 shadow-none focus-visible:ring-0"
                />
              </div>
              <Button type="button" variant="outline" onClick={copyLink}>
                <Copy />
                Copy link
              </Button>
            </div>
            <p
              id={`${ID}-slug-help`}
              className={
                slugError ? "text-sm text-destructive" : "text-sm text-muted-foreground"
              }
            >
              {slugError ??
                "3 to 40 lowercase letters, numbers and dashes. Changing it breaks links you already shared."}
            </p>
          </div>

          <div className="grid gap-1.5">
            <Label htmlFor={`${ID}-title`}>Title</Label>
            <Input
              id={`${ID}-title`}
              required
              maxLength={80}
              value={values.title}
              onChange={(event) => update({ title: event.target.value })}
            />
          </div>

          <div className="grid gap-1.5">
            <Label htmlFor={`${ID}-description`}>
              Description
              <span className="font-normal text-muted-foreground">(optional)</span>
            </Label>
            <Textarea
              id={`${ID}-description`}
              rows={3}
              maxLength={300}
              value={values.description}
              onChange={(event) => update({ description: event.target.value })}
            />
          </div>

          <div className="grid gap-1.5">
            <span className="text-sm leading-none font-medium">Timezone</span>
            <div className="flex flex-wrap items-center gap-x-4 gap-y-2">
              <TimezoneCombobox
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
            <p className="text-sm text-muted-foreground">
              Your weekly hours are read in this timezone. Visitors see times in their
              own.
            </p>
          </div>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>Questions to ask</CardTitle>
          <CardDescription>
            The form always asks for a name, email and phone number. Turn on anything else
            you want to know before a call.
          </CardDescription>
        </CardHeader>
        <CardContent className="grid gap-4">
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
        </CardContent>
      </Card>

      <div className="flex items-center gap-3">
        <Button type="submit" disabled={save.isPending || unchanged}>
          {save.isPending ? "Saving" : "Save changes"}
        </Button>
        {unchanged ? null : (
          <Button type="button" variant="ghost" onClick={() => setValues(saved)}>
            Discard
          </Button>
        )}
      </div>
    </form>
  );
}

function SettingsPage() {
  const query = useQuery({
    queryKey: QUERY_KEYS.hostSettings,
    queryFn: getHostSettings,
  });

  return (
    <>
      <PageHeader
        title="Settings"
        description="Your booking link and what it asks the people who book you."
      />
      {query.isPending ? (
        <Skeleton className="h-96 w-full rounded-xl" />
      ) : query.isError ? (
        <ErrorState
          title="Your settings did not load"
          error={query.error}
          onRetry={() => void query.refetch()}
        />
      ) : (
        <SettingsForm saved={query.data} />
      )}
      <GoogleCalendarCard />
    </>
  );
}
