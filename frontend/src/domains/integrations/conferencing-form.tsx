import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useBlocker } from "@tanstack/react-router";
import { AlertTriangle, Link as LinkIcon, Loader2 } from "lucide-react";
import { useState, type ReactNode } from "react";
import { toast } from "sonner";

import type { ConferencingApp } from "../../bindings/ConferencingApp";
import type { HostSettings } from "../../bindings/HostSettings";
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

const OPTIONS: readonly {
  value: ConferencingApp;
  name: string;
  icon: ReactNode;
  description: string;
}[] = [
  {
    value: CONFERENCING_APP.googleMeet,
    name: "Google Meet",
    icon: <GoogleMeetIcon />,
    description: "A new Meet link for each call, added to the calendar event.",
  },
  {
    value: CONFERENCING_APP.customLink,
    name: "Your own link",
    icon: <LinkIcon className="size-4 text-muted-foreground" />,
    description: "One Zoom, Teams or other room for every call.",
  },
];

/** What the link field is for under each choice. */
const LINK_COPY = {
  [CONFERENCING_APP.googleMeet]: {
    label: "Backup link",
    help: "Sent instead when Google cannot make a Meet link.",
  },
  [CONFERENCING_APP.customLink]: {
    label: "Meeting link",
    help: "Sent to everyone who books, such as a Zoom or Teams room.",
  },
} as const;

/** The host's default place for every call, and their own link. */
export function ConferencingForm({
  saved,
  calendarConnected,
}: {
  saved: HostSettings;
  calendarConnected: boolean;
}) {
  const queryClient = useQueryClient();
  const savedLink = saved.meeting_link ?? "";
  const [app, setApp] = useState<ConferencingApp>(saved.conferencing_app);
  const [link, setLink] = useState(savedLink);

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
  const meetWithoutCalendar = app === CONFERENCING_APP.googleMeet && !calendarConnected;
  const unchanged = app === saved.conferencing_app && trimmed === savedLink;
  const copy = LINK_COPY[app];

  useBlocker({
    shouldBlockFn: ({ current, next }) =>
      !unchanged &&
      current.pathname !== next.pathname &&
      !window.confirm("Leave without saving your changes?"),
    enableBeforeUnload: !unchanged,
  });

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
        aria-label="Meeting app for new calls"
        className="grid gap-3 sm:grid-cols-2"
      >
        {OPTIONS.map((option) => (
          <Label
            key={option.value}
            htmlFor={`${ID}-${option.value}`}
            className="group relative flex cursor-pointer items-start gap-3 rounded-lg border p-4 font-normal transition-[border-color,background-color,box-shadow] duration-200 hover:bg-muted/40 has-[[data-state=checked]]:border-foreground has-[[data-state=checked]]:bg-muted/40 has-[[data-state=checked]]:shadow-xs has-[:focus-visible]:ring-[3px] has-[:focus-visible]:ring-ring/50"
          >
            <span className="grid size-9 shrink-0 place-items-center rounded-md border bg-card transition-transform duration-200 group-has-[[data-state=checked]]:scale-105">
              {option.icon}
            </span>
            <span className="grid min-w-0 flex-1 gap-1">
              <span className="font-medium">{option.name}</span>
              <span className="text-sm text-muted-foreground">{option.description}</span>
            </span>
            <RadioGroupItem
              id={`${ID}-${option.value}`}
              value={option.value}
              className="mt-0.5"
            />
          </Label>
        ))}
      </RadioGroup>

      {meetWithoutCalendar ? (
        <p
          role="status"
          className="flex items-start gap-2 rounded-lg border border-warning/30 bg-warning/10 px-3 py-2.5 text-sm text-warning motion-safe:animate-in motion-safe:fade-in motion-safe:slide-in-from-top-1"
        >
          <AlertTriangle className="mt-0.5 size-4 shrink-0" />
          <span>
            Google Meet needs Google Calendar. Until you connect it above, calls use your
            backup link
            {trimmed ? "." : ", and without one people who book get no link to join."}
          </span>
        </p>
      ) : null}

      <Field
        id={`${ID}-link`}
        label={copy.label}
        optional={app !== CONFERENCING_APP.customLink}
        error={linkInvalid ? "Use a link that starts with https://" : null}
        help={copy.help}
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

      {unchanged ? null : (
        <div className="sticky bottom-4 flex flex-wrap items-center gap-2 rounded-xl border bg-card py-2.5 pr-2.5 pl-4 shadow-lg motion-safe:animate-rise">
          <p className="mr-auto text-sm font-medium">
            {linkMissing ? "Add your meeting link to save" : "Unsaved changes"}
          </p>
          <Button
            type="button"
            variant="ghost"
            onClick={() => {
              setApp(saved.conferencing_app);
              setLink(savedLink);
            }}
          >
            Discard
          </Button>
          <Button type="submit" disabled={save.isPending || linkInvalid || linkMissing}>
            {save.isPending ? <Loader2 className="animate-spin" /> : null}
            {save.isPending ? "Saving" : "Save changes"}
          </Button>
        </div>
      )}
    </form>
  );
}
