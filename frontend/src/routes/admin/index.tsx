import { useQuery, type UseQueryResult } from "@tanstack/react-query";
import { createFileRoute, Link } from "@tanstack/react-router";
import { cn } from "cn";
import { CheckCircle2 } from "lucide-react";

import type { AdminBugReport } from "../../bindings/AdminBugReport";
import type { AdminEmail } from "../../bindings/AdminEmail";
import type { AdminHost } from "../../bindings/AdminHost";
import type { EmailState } from "../../bindings/EmailState";
import type { UserStatus } from "../../bindings/UserStatus";
import type { VisitFunnel } from "../../bindings/VisitFunnel";
import { Button } from "../../components/ui/button";
import { Card } from "../../components/ui/card";
import { Skeleton } from "../../components/ui/skeleton";
import { HOST_GAPS, type HostGap, hostGaps } from "../../domains/admin/host-setup";
import {
  ADMIN_PAGES,
  adminDataQuery,
  VISIT_FUNNEL_ENDPOINT,
} from "../../domains/admin/routes";
import { EmptyState } from "../../domains/shared/EmptyState";
import { PageHeader } from "../../domains/shared/PageHeader";
import { APP_NAME } from "../../lib/config";

export const Route = createFileRoute("/admin/")({
  head: () => ({ meta: [{ title: `Admin | ${APP_NAME}` }] }),
  component: AdminOverviewPage,
});

const DAY_MS = 24 * 60 * 60 * 1000;
const ACTIVE: UserStatus = "Active";
const RETRYING: EmailState = "Retrying";
const OVERDUE: EmailState = "Overdue";

function useAdminData<T>(endpoint: string) {
  return useQuery(adminDataQuery<T>(endpoint));
}

type Attention = {
  key: string;
  count: number;
  /** Written for one and for many: "email keeps failing" / "emails keep failing". */
  one: string;
  many: string;
  urgent?: boolean;
  to: (typeof ADMIN_PAGES)[keyof typeof ADMIN_PAGES]["to"];
  search?: string;
};

function emailsIn(emails: AdminEmail[], state: EmailState): number {
  return emails.filter((email) => email.state === state).length;
}

function hostsWith(hosts: AdminHost[], gap: HostGap): number {
  return hosts.filter(
    (host) => host.status === ACTIVE && hostGaps(host).some((found) => found === gap),
  ).length;
}

function NeedsAttention() {
  const emails = useAdminData<AdminEmail[]>(ADMIN_PAGES.emails.endpoint);
  const hosts = useAdminData<AdminHost[]>(ADMIN_PAGES.hosts.endpoint);
  const bugs = useAdminData<AdminBugReport[]>(ADMIN_PAGES.bugReports.endpoint);
  const sources: { title: string; query: UseQueryResult }[] = [
    { title: ADMIN_PAGES.emails.title, query: emails },
    { title: ADMIN_PAGES.hosts.title, query: hosts },
    { title: ADMIN_PAGES.bugReports.title, query: bugs },
  ];
  const failed = sources.filter((source) => source.query.isError);
  const loading = sources.some((source) => source.query.isPending);

  const since = Date.now() - DAY_MS;
  const items: Attention[] = [
    {
      key: "retrying",
      count: emailsIn(emails.data ?? [], RETRYING),
      one: "email keeps failing to send",
      many: "emails keep failing to send",
      urgent: true,
      to: ADMIN_PAGES.emails.to,
      search: RETRYING,
    },
    {
      key: "overdue",
      count: emailsIn(emails.data ?? [], OVERDUE),
      one: "email is overdue and was never tried",
      many: "emails are overdue and were never tried",
      urgent: true,
      to: ADMIN_PAGES.emails.to,
      search: OVERDUE,
    },
    {
      key: "no-hours",
      count: hostsWith(hosts.data ?? [], HOST_GAPS.noHours),
      one: "host has no hours, so nobody can book them",
      many: "hosts have no hours, so nobody can book them",
      to: ADMIN_PAGES.hosts.to,
      search: HOST_GAPS.noHours,
    },
    {
      key: "no-call-link",
      count: hostsWith(hosts.data ?? [], HOST_GAPS.noCallLink),
      one: "host has no meeting link or calendar",
      many: "hosts have no meeting link or calendar",
      to: ADMIN_PAGES.hosts.to,
      search: HOST_GAPS.noCallLink,
    },
    {
      key: "errors",
      count: (bugs.data ?? []).filter(
        (report) => new Date(report.created).getTime() >= since,
      ).length,
      one: "kind of error in the last 24 hours",
      many: "kinds of error in the last 24 hours",
      to: ADMIN_PAGES.bugReports.to,
    },
  ].filter((item) => item.count > 0);

  return (
    <Card className="gap-0 overflow-hidden py-0">
      <h2 className="border-b px-5 py-3.5 font-semibold">Needs attention</h2>
      <ul className="divide-y">
        {failed.map(({ title, query }) => (
          <li key={title} className="flex items-center gap-3 px-5 py-3 text-sm">
            <span className="mr-auto text-destructive">{title} did not load</span>
            <Button variant="outline" size="sm" onClick={() => void query.refetch()}>
              Try again
            </Button>
          </li>
        ))}
        {items.map((item) => (
          <li key={item.key} className="flex items-center gap-3 px-5 py-3 text-sm">
            <span
              aria-hidden="true"
              className={cn(
                "size-2 shrink-0 rounded-full",
                item.urgent ? "bg-destructive" : "bg-warning",
              )}
            />
            <p className="mr-auto min-w-0">
              <span className="font-semibold tabular-nums">{item.count}</span>{" "}
              {item.count === 1 ? item.one : item.many}
            </p>
            <Button variant="outline" size="sm" asChild>
              <Link to={item.to} search={item.search ? { q: item.search } : undefined}>
                View
              </Link>
            </Button>
          </li>
        ))}
        {loading ? (
          <li className="px-5 py-3.5">
            <Skeleton className="h-5 w-64" />
          </li>
        ) : null}
      </ul>
      {!loading && failed.length === 0 && items.length === 0 ? (
        <EmptyState
          icon={CheckCircle2}
          title="Nothing needs attention"
          description="Emails are going out, every host can be booked and no errors came in today."
        />
      ) : null}
    </Card>
  );
}

function Funnel() {
  const funnel = useAdminData<VisitFunnel>(VISIT_FUNNEL_ENDPOINT);
  const visits = funnel.data?.visits ?? 0;
  const steps = [
    { label: "Visited a booking page", count: visits },
    { label: "Sent the form", count: funnel.data?.form_submitted ?? 0 },
    { label: "Booked a call", count: funnel.data?.booked ?? 0 },
  ];

  return (
    <Card className="gap-0 overflow-hidden py-0">
      <h2 className="border-b px-5 py-3.5 font-semibold">Last 7 days</h2>
      {funnel.isError ? (
        <div className="flex items-center gap-3 px-5 py-3 text-sm">
          <span className="mr-auto text-destructive">Visits did not load</span>
          <Button variant="outline" size="sm" onClick={() => void funnel.refetch()}>
            Try again
          </Button>
        </div>
      ) : (
        <dl className="grid gap-4 px-5 py-4">
          {steps.map((step) => (
            <div key={step.label} className="grid gap-1.5">
              <div className="flex items-baseline justify-between gap-3 text-sm">
                <dt className="text-muted-foreground">{step.label}</dt>
                <dd className="font-display text-lg font-semibold tabular-nums">
                  {funnel.isPending ? <Skeleton className="h-6 w-10" /> : step.count}
                </dd>
              </div>
              <div
                aria-hidden="true"
                className="h-1.5 overflow-hidden rounded-full bg-muted"
              >
                {funnel.isPending ? null : (
                  <div
                    className="h-full origin-left rounded-full bg-primary motion-safe:animate-bar-grow"
                    style={{ width: `${visits ? (step.count / visits) * 100 : 0}%` }}
                  />
                )}
              </div>
            </div>
          ))}
        </dl>
      )}
    </Card>
  );
}

function AdminOverviewPage() {
  return (
    <>
      <PageHeader
        title="Admin"
        description="What needs a look right now, and how the week is going."
      />
      <div className="grid items-start gap-6 lg:grid-cols-[minmax(0,3fr)_minmax(0,2fr)]">
        <NeedsAttention />
        <Funnel />
      </div>
    </>
  );
}
