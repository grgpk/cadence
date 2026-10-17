import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { createFileRoute, Link, useNavigate } from "@tanstack/react-router";
import { ChevronRight, Copy, Import, Loader2, Plus } from "lucide-react";
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

function TypeRow({ kind, hostSlug }: { kind: EventType; hostSlug: string }) {
  const path = `${hostSlug}/${kind.slug}`;
  return (
    <Card className="flex-row items-center gap-4 px-5 py-4">
      <div className="min-w-0 flex-1">
        <div className="flex flex-wrap items-center gap-2">
          <Link
            to={ROUTES.dashboardEventType}
            params={{ unid: kind.unid }}
            className="font-semibold underline-offset-4 outline-none hover:underline focus-visible:ring-[3px] focus-visible:ring-ring/50"
          >
            {kind.title}
          </Link>
          {!kind.is_active ? (
            <Badge variant="outline">Off</Badge>
          ) : !kind.bookable ? (
            <Badge variant="outline">Needs Pro</Badge>
          ) : null}
        </div>
        <p className="mt-1 truncate text-sm text-muted-foreground">
          {bookingPath(path)} · {lengthText(kind.duration_minutes)}
        </p>
      </div>
      <Button variant="outline" size="sm" onClick={() => copyBookingLink(path)}>
        <Copy />
        Copy link
      </Button>
      <Button variant="ghost" size="icon" asChild>
        <Link
          to={ROUTES.dashboardEventType}
          params={{ unid: kind.unid }}
          aria-label={`Edit ${kind.title}`}
        >
          <ChevronRight />
        </Link>
      </Button>
    </Card>
  );
}

/** A `TypeRow` while it loads: title, link line and the two buttons. */
function TypeRowSkeleton() {
  return (
    <Card className="flex-row items-center gap-4 px-5 py-4">
      <div className="grid min-w-0 flex-1 gap-2">
        <Skeleton className="h-5 w-40" />
        <Skeleton className="h-4 w-64 max-w-full" />
      </div>
      <Skeleton className="h-8 w-28" />
      <Skeleton className="size-9" />
    </Card>
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
        <div className="grid gap-3">
          {types.data.map((kind) => (
            <TypeRow key={kind.unid} kind={kind} hostSlug={settings.data.public_slug} />
          ))}
        </div>
      ) : (
        <div className="grid gap-3" role="status" aria-label="Loading">
          {[0, 1].map((index) => (
            <TypeRowSkeleton key={index} />
          ))}
        </div>
      )}
    </>
  );
}
