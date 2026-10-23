import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useBlocker, useNavigate } from "@tanstack/react-router";
import { Copy, Loader2, Trash2 } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { toast } from "sonner";

import type { EventType } from "../../bindings/EventType";
import type { EventTypeInput } from "../../bindings/EventTypeInput";
import { Button } from "../../components/ui/button";
import { Input } from "../../components/ui/input";
import { Label } from "../../components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "../../components/ui/select";
import { Switch } from "../../components/ui/switch";
import { Textarea } from "../../components/ui/textarea";
import { ApiError } from "../../lib/api";
import { errorMessage } from "../../lib/error-message";
import { QUERY_KEYS } from "../../lib/query-keys";
import { bookingPath, ROUTES } from "../../lib/routes";
import { SLOT_MINUTES } from "../availability/hours-form";
import { billingQuery } from "../billing/billing-api";
import { PlanNotice } from "../billing/plan-notice";
import { copyBookingLink } from "../dashboard/booking-link";
import { ConfirmDialog } from "../shared/confirm-dialog";
import { Field, fieldHelpId } from "../shared/field";
import { PanelCard } from "../shared/panel-card";
import { QUESTIONS } from "../widget/booking-form";
import { deleteEventType, saveEventType } from "./event-types-api";
import { TypePreview } from "./type-preview";
import {
  BUFFER_MINUTES,
  inputOf,
  lengthText,
  NOTICE_HOURS,
  SLUG_PATTERN,
  WINDOW_DAYS,
  withQuestion,
} from "./event-types";

const ID = "event-type";
/** The length choice that leaves it to the host's hours. */
const FROM_HOURS = "hours";

/** A number field's value: empty while the host is between two numbers. */
function numberValue(value: number): number | "" {
  return Number.isNaN(value) ? "" : value;
}

/** Edits one kind of call. `hostSlug` is the host's own link. */
export function EventTypeForm({
  saved,
  hostName,
  accent,
  hostSlug,
  isLast,
}: {
  saved: EventType;
  /** Shown in the preview, as visitors see it. */
  hostName: string;
  accent: string;
  hostSlug: string;
  /** The host's only type cannot be deleted. */
  isLast: boolean;
}) {
  const queryClient = useQueryClient();
  const navigate = useNavigate();
  const original = inputOf(saved);
  const [values, setValues] = useState<EventTypeInput>(original);
  const [slugError, setSlugError] = useState<string | null>(null);
  const [confirmingDelete, setConfirmingDelete] = useState(false);
  const update = (patch: Partial<EventTypeInput>) => setValues({ ...values, ...patch });
  // Questions saved under a paid plan stay saved and can be turned off;
  // turning on a new one needs a plan with questions.
  const billing = useQuery(billingQuery);
  const questionsLocked = billing.data?.entitlements.custom_questions === false;

  // The type can change behind the form (a refetch, another tab). Follow it
  // while nothing here has been edited; local edits are never thrown away.
  const followed = useRef(original);
  useEffect(() => {
    const next = inputOf(saved);
    const before = followed.current;
    followed.current = next;
    setValues((current) =>
      JSON.stringify(current) === JSON.stringify(before) ? next : current,
    );
  }, [saved]);

  const refreshPublic = () =>
    Promise.all([
      queryClient.invalidateQueries({ queryKey: QUERY_KEYS.eventTypes }),
      queryClient.invalidateQueries({ queryKey: QUERY_KEYS.hostPage(hostSlug) }),
      queryClient.invalidateQueries({ queryKey: QUERY_KEYS.bookingPage(hostSlug) }),
    ]);

  const save = useMutation({
    mutationFn: (input: EventTypeInput) => saveEventType(saved.unid, input),
    onSuccess: async (next) => {
      queryClient.setQueryData<EventType[]>(QUERY_KEYS.eventTypes, (types) =>
        types?.map((kind) => (kind.unid === next.unid ? next : kind)),
      );
      await refreshPublic();
      setValues(inputOf(next));
      setSlugError(null);
      toast.success("Changes saved");
    },
    onError: (error) => {
      if (error instanceof ApiError && error.isConflict) {
        setSlugError("You already have a call at this link. Try another one.");
        document.getElementById(`${ID}-slug`)?.focus();
        return;
      }
      toast.error(
        `Changes were not saved. ${errorMessage(error, {
          403: "Your plan does not include your own booking questions.",
        })}`,
      );
    },
  });

  const remove = useMutation({
    mutationFn: () => deleteEventType(saved.unid),
    onSuccess: async () => {
      await refreshPublic();
      toast.success(`${saved.title} was deleted`);
      void navigate({ to: ROUTES.dashboardEventTypes });
    },
    onError: (error) => toast.error(`It was not deleted. ${errorMessage(error)}`),
  });

  const unchanged = JSON.stringify(values) === JSON.stringify(original);
  // Leaving the page would drop the edits without a word.
  useBlocker({
    shouldBlockFn: ({ current, next }) =>
      !unchanged &&
      !remove.isSuccess &&
      current.pathname !== next.pathname &&
      !window.confirm("Leave without saving your changes?"),
    enableBeforeUnload: !unchanged,
  });
  const linkPrefix = `${window.location.host}${bookingPath(hostSlug)}/`;

  return (
    <div className="grid items-start gap-8 xl:grid-cols-[minmax(0,42rem)_minmax(0,1fr)]">
      <form
        className="grid min-w-0 gap-6"
        onSubmit={(event) => {
          event.preventDefault();
          save.mutate(values);
        }}
      >
        <PanelCard
          title="Details"
          description="What people read when they pick this call."
        >
          <Field id={`${ID}-title`} label="Title">
            <Input
              id={`${ID}-title`}
              required
              maxLength={80}
              value={values.title}
              onChange={(event) => update({ title: event.target.value })}
            />
          </Field>
          <Field
            id={`${ID}-slug`}
            label="Link"
            error={slugError}
            help="2 to 40 lowercase letters, numbers and dashes. Changing it breaks links you already shared."
          >
            <div className="flex flex-wrap items-center gap-2">
              <div className="flex min-w-0 flex-1 items-center rounded-md border border-input bg-muted shadow-xs focus-within:border-ring focus-within:ring-[3px] focus-within:ring-ring/50">
                <span className="truncate pl-3 text-sm text-muted-foreground">
                  {linkPrefix}
                </span>
                <Input
                  id={`${ID}-slug`}
                  value={values.slug}
                  required
                  minLength={2}
                  maxLength={40}
                  pattern={SLUG_PATTERN}
                  title="2 to 40 lowercase letters, numbers and single dashes"
                  spellCheck={false}
                  autoCapitalize="none"
                  aria-invalid={Boolean(slugError)}
                  aria-describedby={fieldHelpId(`${ID}-slug`)}
                  onChange={(event) => {
                    setSlugError(null);
                    update({ slug: event.target.value.toLowerCase() });
                  }}
                  className="min-w-24 flex-1 rounded-l-none border-0 bg-card pl-1.5 shadow-none focus-visible:ring-0"
                />
              </div>
              <Button
                type="button"
                variant="outline"
                onClick={() => copyBookingLink(`${hostSlug}/${original.slug}`)}
              >
                <Copy />
                Copy link
              </Button>
            </div>
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
          <Field
            id={`${ID}-length`}
            label="Length"
            help="Open times step through your weekly hours at this length. From your hours keeps each range's own length."
          >
            <Select
              value={
                values.duration_minutes === null
                  ? FROM_HOURS
                  : String(values.duration_minutes)
              }
              onValueChange={(value) =>
                update({ duration_minutes: value === FROM_HOURS ? null : Number(value) })
              }
            >
              <SelectTrigger
                id={`${ID}-length`}
                className="w-56 bg-card"
                aria-describedby={fieldHelpId(`${ID}-length`)}
              >
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value={FROM_HOURS}>{lengthText(null)}</SelectItem>
                {SLOT_MINUTES.map((minutes) => (
                  <SelectItem key={minutes} value={String(minutes)}>
                    {lengthText(minutes)}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </Field>
        </PanelCard>

        <PanelCard
          title="Booking rules"
          description="How soon and how far ahead people can book this call."
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
                  className="w-24 text-clock text-md"
                />
                <span className="text-sm text-muted-foreground">hours</span>
              </div>
            </Field>
            <Field
              id={`${ID}-window`}
              label="Booking window"
              help="How far ahead this call shows times."
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
                  className="w-24 text-clock text-md"
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
                  className="w-24 text-clock text-md"
                />
                <span className="text-sm text-muted-foreground">minutes</span>
              </div>
            </Field>
          </div>
          <div className="flex items-center justify-between gap-4">
            <div>
              <Label htmlFor={`${ID}-active`}>Offer this call</Label>
              <p className="mt-1 text-sm text-muted-foreground">
                Turn off to take it off your link. It stays saved, and calls already
                booked stay.
              </p>
            </div>
            <Switch
              id={`${ID}-active`}
              checked={values.is_active}
              onCheckedChange={(checked) => update({ is_active: checked })}
            />
          </div>
        </PanelCard>

        <PanelCard
          title="Questions to ask"
          description="The form always asks for a name, email and phone number. Turn on anything else you want to know before this call."
        >
          <div className="grid gap-4">
            {questionsLocked ? (
              <PlanNotice>
                Your own questions come with Pro. Any you set before stay saved and are
                asked again when you upgrade.
              </PlanNotice>
            ) : null}
            {QUESTIONS.map((question) => (
              <div key={question.key} className="flex items-center justify-between gap-4">
                <Label htmlFor={`${ID}-${question.key}`} className="font-normal">
                  {question.label}
                </Label>
                <Switch
                  id={`${ID}-${question.key}`}
                  checked={values.questions.includes(question.key)}
                  disabled={questionsLocked && !original.questions.includes(question.key)}
                  onCheckedChange={(checked) =>
                    update({
                      questions: withQuestion(values.questions, question.key, checked),
                    })
                  }
                />
              </div>
            ))}
          </div>
        </PanelCard>

        <div className="flex flex-wrap items-center justify-between gap-3 rounded-xl border border-destructive/30 px-5 py-4">
          <div>
            <p className="font-medium">Delete this call</p>
            <p className="text-sm text-muted-foreground">
              {isLast
                ? "Your page needs one kind of call. Add another before deleting this one."
                : "Calls already booked stay, under your first kind of call."}
            </p>
          </div>
          <Button
            type="button"
            variant="outline"
            className="text-destructive"
            disabled={isLast || remove.isPending}
            onClick={() => setConfirmingDelete(true)}
          >
            <Trash2 />
            Delete
          </Button>
        </div>

        {unchanged ? null : (
          <div className="sticky bottom-4 flex flex-wrap items-center gap-2 rounded-xl border bg-card py-2.5 pr-2.5 pl-4 shadow-lg motion-safe:animate-rise">
            <p className="mr-auto text-sm font-medium">Unsaved changes</p>
            <Button
              type="button"
              variant="ghost"
              onClick={() => {
                setValues(original);
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
          open={confirmingDelete}
          onOpenChange={setConfirmingDelete}
          title={`Delete ${saved.title}?`}
          description={`${linkPrefix}${saved.slug} stops working. Calls already booked stay.`}
          confirmLabel="Delete"
          destructive
          onConfirm={() => remove.mutate()}
        />
      </form>
      <aside className="sticky top-20 max-xl:hidden" aria-label="Preview">
        <TypePreview
          values={values}
          hostName={hostName}
          accent={accent}
          hostSlug={hostSlug}
          savedSlug={original.slug}
        />
      </aside>
    </div>
  );
}
