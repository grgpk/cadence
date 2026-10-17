import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { createFileRoute, Link, useNavigate } from "@tanstack/react-router";
import { ChevronRight, Clock, Copy, Import, Loader2, Plus } from "lucide-react";
import { toast } from "sonner";

import type { EventType } from "../../../bindings/EventType";
import { Badge } from "../../../components/ui/badge";
import { Button } from "../../../components/ui/button";
import { Card } from "../../../components/ui/card";
import { Skeleton } from "../../../components/ui/skeleton";
import { billingQuery } from "../../../domains/billing/billing-api";
import { PlanNotice } from "../../../domains/billing/plan-notice";
import { copyBookingLink } from "../../../domains/dashboard/booking-link";
import { hostSettingsQuery } from "../../../domains/dashboard/host-api";
import {
  canAddType,
  lengthText,
  newTypeInput,
} from "../../../domains/event-types/event-types";
import {
  createEventType,
  eventTypesQuery,
} from "../../../domains/event-types/event-types-api";
import { PLAN_NAMES } from "../../../domains/pricing/pricing";
import { ErrorState } from "../../../domains/shared/error-state";
import { PageHeader } from "../../../domains/shared/page-header";
import { APP_NAME } from "../../../lib/config";
import { errorMessage } from "../../../lib/error-message";
import { QUERY_KEYS } from "../../../lib/query-keys";
import { bookingPath, ROUTES } from "../../../lib/routes";

export const Route = createFileRoute("/dashboard/event-types/")({
  head: () => ({ meta: [{ title: `Event types | ${APP_NAME}` }] }),
  component: EventTypesPage,
});

/** One kind of call. The whole row opens it; copying the link stays its own button. */
function TypeRow({ kind, hostSlug }: { kind: EventType; hostSlug: string }) {
  const path = `${hostSlug}/${kind.slug}`;
  return (
    <li className="group relative flex min-w-0 items-center gap-3 px-4 py-4 transition-colors hover:bg-muted/40 sm:gap-4 sm:px-5">
      <div className="min-w-0 flex-1">
        <Link
          to={ROUTES.dashboardEventType}
          params={{ unid: kind.unid }}
          className="font-semibold outline-none after:absolute after:inset-0 focus-visible:after:rounded-[inherit] focus-visible:after:ring-[3px] focus-visible:after:ring-ring/50 focus-visible:after:ring-inset"
        >
          {kind.title}
        </Link>
        <p className="mt-0.5 truncate text-sm text-muted-foreground">
          {bookingPath(path)}
        </p>
        <div className="mt-2 flex flex-wrap items-center gap-1.5">
          <span className="inline-flex items-center gap-1.5 rounded-full border px-2.5 py-0.5 text-xs font-medium text-muted-foreground">
            <Clock className="size-3.5" aria-hidden />
            {lengthText(kind.duration_minutes)}
          </span>
          {!kind.is_active ? (
            <Badge variant="outline">Off</Badge>
          ) : !kind.bookable ? (
            <Badge variant="outline">Needs Pro</Badge>
          ) : null}
        </div>
      </div>
      <Button
        variant="outline"
        size="sm"
        className="relative z-10"
        aria-label={`Copy link to ${kind.title}`}
        onClick={() => copyBookingLink(path)}
      >
        <Copy />
        <span className="max-sm:hidden">Copy link</span>
      </Button>
      <ChevronRight
        aria-hidden
        className="size-5 shrink-0 text-muted-foreground transition-transform group-hover:translate-x-0.5 group-hover:text-foreground"
      />
    </li>
  );
}

/** A `TypeRow` while it loads: title, link line, chip and the copy button. */
function TypeRowSkeleton() {
  return (
    <li className="flex items-center gap-4 px-4 py-4 sm:px-5">
      <div className="grid min-w-0 flex-1 gap-2">
        <Skeleton className="h-5 w-40" />
        <Skeleton className="h-4 w-64 max-w-full" />
        <Skeleton className="h-5 w-20 rounded-full" />
      </div>
      <Skeleton className="h-8 w-9 sm:w-28" />
    </li>
  );
}

function EventTypesPage() {
  const queryClient = useQueryClient();
  const navigate = useNavigate();
  const types = useQuery(eventTypesQuery);
  const settings = useQuery(hostSettingsQuery);
  const billing = useQuery(billingQuery);
  const canAdd =
    billing.data && types.data
      ? canAddType(billing.data.entitlements, types.data.length)
      : false;

  const create = useMutation({
    mutationFn: () => createEventType(newTypeInput(types.data ?? [])),
    onSuccess: async (created) => {
      await queryClient.invalidateQueries({ queryKey: QUERY_KEYS.eventTypes });
      void navigate({ to: ROUTES.dashboardEventType, params: { unid: created.unid } });
    },
    onError: (error) =>
      toast.error(
        `The call was not added. ${errorMessage(error, {
          403: "Your plan does not include more event types.",
        })}`,
      ),
  });

  const failed = types.isError ? types : settings.isError ? settings : null;
  return (
    <>
      <PageHeader
        title="Event types"
        description="The kinds of call people can book from your link. With one, your link opens it straight away; with several, people pick one."
        actions={
          <>
            <Button variant="outline" asChild>
              <Link to={ROUTES.dashboardImportCalendly}>
                <Import />
                Import from Calendly
              </Link>
            </Button>
            <Button
              onClick={() => create.mutate()}
              disabled={!canAdd || create.isPending}
            >
              {create.isPending ? <Loader2 className="animate-spin" /> : <Plus />}
              New event type
            </Button>
          </>
        }
      />
      {billing.data && types.data && !canAdd ? (
        <PlanNotice plan={PLAN_NAMES.pro}>
          Several kinds of call come with Pro. Your other calls stay saved.
        </PlanNotice>
      ) : null}
      {failed ? (
        <ErrorState
          title="Your event types did not load"
          error={failed.error}
          onRetry={() => void failed.refetch()}
        />
      ) : types.data && settings.data ? (
        <Card className="min-w-0 gap-0 overflow-hidden py-0">
          <ul className="divide-y">
            {types.data.map((kind) => (
              <TypeRow key={kind.unid} kind={kind} hostSlug={settings.data.public_slug} />
            ))}
          </ul>
        </Card>
      ) : (
        <Card className="gap-0 overflow-hidden py-0" role="status" aria-label="Loading">
          <ul className="divide-y">
            {[0, 1].map((index) => (
              <TypeRowSkeleton key={index} />
            ))}
          </ul>
        </Card>
      )}
    </>
  );
}
