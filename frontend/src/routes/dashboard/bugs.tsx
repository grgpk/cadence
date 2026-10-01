import { useQuery } from "@tanstack/react-query";
import { createFileRoute } from "@tanstack/react-router";
import { Bug, CircleAlert, Inbox } from "lucide-react";

import type { AdminBugReport } from "../../bindings/AdminBugReport";
import { Alert, AlertDescription } from "../../components/ui/alert";
import { Card, CardContent, CardHeader, CardTitle } from "../../components/ui/card";
import { Skeleton } from "../../components/ui/skeleton";
import { api } from "../../lib/api";
import { QUERY_KEYS } from "../../lib/query-keys";

export const Route = createFileRoute("/dashboard/bugs")({ component: BugsPage });

function BugsPage() {
  const {
    data: bugs = [],
    error,
    isPending,
  } = useQuery({
    queryKey: QUERY_KEYS.bugs,
    queryFn: () => api<AdminBugReport[]>("/api/admin/bug-reports"),
  });

  return (
    <section className="flex flex-1 flex-col gap-6">
      <header>
        <p className="text-sm font-medium text-muted-foreground">Workspace</p>
        <h1 className="text-2xl font-semibold tracking-tight">Bug reports</h1>
        <p className="text-muted-foreground">Application errors captured by Cadence.</p>
      </header>
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Bug className="size-5 text-muted-foreground" />
            Recent reports
          </CardTitle>
        </CardHeader>
        <CardContent>
          {isPending ? (
            <div className="space-y-4">
              <Skeleton className="h-16 w-full" />
              <Skeleton className="h-16 w-full" />
            </div>
          ) : error ? (
            <Alert variant="destructive">
              <CircleAlert className="size-4" />
              <AlertDescription>{error.message}</AlertDescription>
            </Alert>
          ) : bugs.length === 0 ? (
            <div className="flex flex-col items-center gap-2 py-12 text-center text-muted-foreground">
              <Inbox className="size-8" />
              <p>No bug reports.</p>
            </div>
          ) : (
            <div className="divide-y">
              {bugs.map((bug) => (
                <article key={bug.unid} className="py-4 first:pt-0 last:pb-0">
                  <div className="flex flex-wrap items-center justify-between gap-3">
                    <p className="font-medium">{bug.bugtype}</p>
                    <time className="text-sm text-muted-foreground">
                      {new Date(bug.created).toLocaleString()}
                    </time>
                  </div>
                  <p className="mt-2 text-sm text-muted-foreground">
                    {bug.message ?? bug.exceptionmessage ?? "No message"}
                  </p>
                </article>
              ))}
            </div>
          )}
        </CardContent>
      </Card>
    </section>
  );
}
