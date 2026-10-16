import { useMutation, useQueryClient } from "@tanstack/react-query";
import { Link as LinkIcon, Loader2 } from "lucide-react";
import { useState } from "react";
import { toast } from "sonner";

import type { ConferencingApp } from "../../bindings/ConferencingApp";
import type { HostSettings } from "../../bindings/HostSettings";
import { Badge } from "../../components/ui/badge";
import { Button } from "../../components/ui/button";
import { Input } from "../../components/ui/input";
import { Label } from "../../components/ui/label";
import { RadioGroup, RadioGroupItem } from "../../components/ui/radio-group";
import { errorMessage } from "../../lib/error-message";
import { QUERY_KEYS } from "../../lib/query-keys";
import { saveConferencing } from "../dashboard/host-api";
import { Field, fieldHelpId } from "../shared/field";
import { CONFERENCING_APP } from "./conferencing";
import { GoogleMeetIcon } from "./google-meet-icon";

const ID = "conferencing";
const SECURE_LINK_PREFIX = "https://";

/** The host's default place for every call, and their own link. */
export function ConferencingForm({
  saved,
  calendarConnected,
}: {
  saved: HostSettings;
  calendarConnected: boolean;
}) {
  const queryClient = useQueryClient();
  const [app, setApp] = useState<ConferencingApp>(saved.conferencing_app);
  const [link, setLink] = useState(saved.meeting_link ?? "");

  const save = useMutation({
    mutationFn: saveConferencing,
    onSuccess: async (next) => {
      queryClient.setQueryData(QUERY_KEYS.hostSettings, next);
      await queryClient.invalidateQueries({
        queryKey: QUERY_KEYS.bookingPage(next.public_slug),
      });
      setApp(next.conferencing_app);
      setLink(next.meeting_link ?? "");
      toast.success("Changes saved");
    },
    onError: (error) => toast.error(`Changes were not saved. ${errorMessage(error)}`),
  });

  const trimmed = link.trim();
  const linkInvalid = trimmed !== "" && !trimmed.startsWith(SECURE_LINK_PREFIX);
  const linkMissing = app === CONFERENCING_APP.customLink && trimmed === "";
  const meetUnavailable = app === CONFERENCING_APP.googleMeet && !calendarConnected;
  const unchanged =
    app === saved.conferencing_app && trimmed === (saved.meeting_link ?? "");

  const options = [
    {
      value: CONFERENCING_APP.googleMeet,
      name: "Google Meet",
      icon: <GoogleMeetIcon />,
      description: "A new Meet link for each call, made with the calendar event.",
    },
    {
      value: CONFERENCING_APP.customLink,
      name: "Your own link",
      icon: <LinkIcon className="size-4 text-muted-foreground" />,
      description: "The same Zoom, Teams or other room for every call.",
    },
  ] as const;

  return (
    <form
      className="grid gap-5"
      onSubmit={(event) => {
        event.preventDefault();
        save.mutate({ app, meeting_link: trimmed || null });
      }}
    >
      <RadioGroup
        value={app}
        onValueChange={(value) => setApp(value as ConferencingApp)}
        aria-label="Default meeting app"
        className="grid gap-3 sm:grid-cols-2"
      >
        {options.map((option) => (
          <Label
            key={option.value}
            htmlFor={`${ID}-${option.value}`}
            className="flex cursor-pointer items-start gap-3 rounded-lg border p-4 font-normal has-[[data-state=checked]]:border-primary"
          >
            <RadioGroupItem
              id={`${ID}-${option.value}`}
              value={option.value}
              className="mt-0.5"
            />
            <span className="grid gap-1">
              <span className="flex items-center gap-2 font-medium">
                {option.icon}
                {option.name}
                {option.value === saved.conferencing_app ? (
                  <Badge variant="outline">Default</Badge>
                ) : null}
              </span>
              <span className="text-sm text-muted-foreground">{option.description}</span>
            </span>
          </Label>
        ))}
      </RadioGroup>

      {meetUnavailable ? (
        <p className="text-sm text-warning">
          Google Meet needs Google Calendar. Until you connect it above, calls use your
          own link{trimmed ? "" : ", and without one people who book get no link"}.
        </p>
      ) : null}

      <Field
        id={`${ID}-link`}
        label="Meeting link"
        optional={app !== CONFERENCING_APP.customLink}
        error={linkInvalid ? "Use a link that starts with https://" : null}
        help="Such as a Zoom or Teams room. Also used when Google does not make a Meet link."
      >
        <Input
          id={`${ID}-link`}
          type="url"
          inputMode="url"
          maxLength={500}
          required={app === CONFERENCING_APP.customLink}
          placeholder="https://zoom.us/j/123456789"
          value={link}
          aria-invalid={linkInvalid}
          aria-describedby={fieldHelpId(`${ID}-link`)}
          onChange={(event) => setLink(event.target.value)}
        />
      </Field>

      <div className="flex justify-end">
        <Button
          type="submit"
          disabled={unchanged || save.isPending || linkInvalid || linkMissing}
        >
          {save.isPending ? <Loader2 className="animate-spin" /> : null}
          {save.isPending ? "Saving" : "Save"}
        </Button>
      </div>
    </form>
  );
}
