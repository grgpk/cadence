import { useQuery } from "@tanstack/react-query";
import { createFileRoute, Link } from "@tanstack/react-router";

import type { AvailabilityRule } from "../../bindings/AvailabilityRule";
import { Button } from "../../components/ui/button";
import { Card } from "../../components/ui/card";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "../../components/ui/tabs";
import { DatesCalendar } from "../../domains/availability/dates-calendar";
import { hoursLabel, roomyWeek, weekTotals } from "../../domains/availability/hours-math";
import { WeekCanvas } from "../../domains/availability/week-canvas";
import { daysLabel } from "../../domains/dashboard/hours-labels";
import {
  availabilityOverridesQuery,
  availabilityQuery,
  hostSettingsQuery,
} from "../../domains/dashboard/host-api";
import { SETTINGS_SECTIONS } from "../../domains/dashboard/routes";
import { weekHours } from "../../domains/dashboard/week-hours";
import { ErrorState } from "../../domains/shared/error-state";
import { PageHeader } from "../../domains/shared/page-header";
import { PanelHeader } from "../../domains/shared/panel-card";
import { TableSkeleton } from "../../domains/shared/table-skeleton";
import { APP_NAME } from "../../lib/config";
import { dayKey } from "../../lib/civil-day";
import { zoneName } from "../../lib/format";
import { zonedTime } from "../../lib/zoned-time";
import { ROUTES } from "../../lib/routes";

const TAB = { week: "week", dates: "dates" } as const;
type AvailabilitySearch = { tab?: typeof TAB.dates };

export const Route = createFileRoute("/dashboard/availability")({
  // The open tab lives in the URL, so a reload stays on Dates.
  validateSearch: (search): AvailabilitySearch =>
    search.tab === TAB.dates ? { tab: TAB.dates } : {},
  head: () => ({ meta: [{ title: `Availability | ${APP_NAME}` }] }),
  component: AvailabilityPage,
});

/** "Open 40 hours a week, Mon to Fri: up to 80 calls of 30 min." */
function weekSummary(rules: AvailabilityRule[]): string {
  if (rules.length === 0)
    return "No weekly hours yet. Drag across a day to open it for calls.";
  const { minutes, calls } = weekTotals(rules);
  const lengths = [...new Set(rules.map((rule) => rule.slot_minutes))]
    .sort((a, b) => a - b)
    .join(" or ");
  // "Every day" starts a label, not this sentence.
  const days = daysLabel(rules).replace(/^Every/, "every");
  return `Open ${hoursLabel(minutes)} a week, ${days}: up to ${calls} calls of ${lengths} min.`;
}

function AvailabilityPage() {
  const query = useQuery(availabilityQuery);
  const overrides = useQuery(availabilityOverridesQuery);
  const settings = useQuery(hostSettingsQuery);
  const { tab } = Route.useSearch();
  const navigate = Route.useNavigate();
  const timezone = settings.data?.timezone;
  const week = roomyWeek(weekHours(query.data ?? []));
  const today = timezone ? dayKey(zonedTime(new Date(), timezone)) : null;
  const setApart = today
    ? overrides.data?.filter((day) => day.date >= today).length
    : undefined;

  return (
    <>
      <PageHeader
        title="Availability"
        description={`${query.data ? weekSummary(query.data) : "The hours people can book a call with you."}${
          timezone ? ` Times are in ${zoneName(timezone)}.` : ""
        }`}
        actions={
          <Button variant="outline" size="sm" asChild>
            <Link to={ROUTES.dashboardSettings} hash={SETTINGS_SECTIONS.time}>
              Change timezone
            </Link>
          </Button>
        }
      />
      {query.isError ? (
        <ErrorState
          title="Your hours did not load"
          error={query.error}
          onRetry={() => void query.refetch()}
        />
      ) : (
        <Tabs
          value={tab ?? TAB.week}
          onValueChange={(next) =>
            void navigate({
              search: next === TAB.dates ? { tab: TAB.dates } : {},
              replace: true,
            })
          }
        >
          <TabsList className="max-sm:w-full">
            <TabsTrigger value={TAB.week}>Weekly hours</TabsTrigger>
            <TabsTrigger value={TAB.dates}>
              Dates
              {setApart ? (
                <span className="text-muted-foreground tabular-nums">{setApart}</span>
              ) : null}
            </TabsTrigger>
          </TabsList>
          <TabsContent value={TAB.week} className="motion-safe:animate-step-in">
            <Card className="gap-0 py-0">
              <PanelHeader
                title="Weekly hours"
                description="Drag across a day to open it. Drag a range to move it, or its ends to stretch it."
              />
              {query.isPending ? (
                <TableSkeleton rows={7} />
              ) : (
                <WeekCanvas rules={query.data} week={week} />
              )}
            </Card>
          </TabsContent>
          <TabsContent value={TAB.dates} className="motion-safe:animate-step-in">
            <Card className="gap-0 py-0">
              <PanelHeader
                title="Dates"
                description="Take a date off, or give it other hours. Every other date follows the weekly hours."
              />
              {overrides.isError ? (
                <div className="p-5">
                  <ErrorState
                    title="Your dates did not load"
                    error={overrides.error}
                    onRetry={() => void overrides.refetch()}
                  />
                </div>
              ) : query.isPending || overrides.isPending || !timezone ? (
                <TableSkeleton rows={5} />
              ) : (
                <DatesCalendar
                  rules={query.data}
                  overrides={overrides.data}
                  timezone={timezone}
                  week={week}
                />
              )}
            </Card>
          </TabsContent>
        </Tabs>
      )}
    </>
  );
}
