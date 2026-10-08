import { useQuery } from "@tanstack/react-query";
import type { LucideIcon } from "lucide-react";

import { Card } from "../../../components/ui/card";
import { api } from "../../../lib/api";
import { QUERY_KEYS } from "../../../lib/query-keys";
import { DataTable, type Column } from "../../shared/DataTable";
import { EmptyState } from "../../shared/EmptyState";
import { ErrorState } from "../../shared/ErrorState";
import { PageHeader } from "../../shared/PageHeader";
import { TableSkeleton } from "../../shared/TableSkeleton";

type AdminDataPageProps<T> = {
  title: string;
  description: string;
  icon: LucideIcon;
  endpoint: string;
  emptyTitle: string;
  emptyDescription: string;
  rowKey: (item: T) => string;
  columns: Column<T>[];
};

export function AdminDataPage<T>({
  title,
  description,
  icon,
  endpoint,
  emptyTitle,
  emptyDescription,
  rowKey,
  columns,
}: AdminDataPageProps<T>) {
  const query = useQuery({
    queryKey: QUERY_KEYS.adminData(endpoint),
    queryFn: () => api<T[]>(endpoint),
  });

  return (
    <>
      <PageHeader
        title={title}
        description={description}
        actions={
          query.data ? (
            <p className="text-sm text-muted-foreground tabular-nums">
              {query.data.length} {query.data.length === 1 ? "record" : "records"}
            </p>
          ) : null
        }
      />
      {query.isError ? (
        <ErrorState
          title={`${title} did not load`}
          error={query.error}
          onRetry={() => void query.refetch()}
        />
      ) : (
        <Card className="gap-0 overflow-hidden py-0">
          {query.isPending ? (
            <TableSkeleton />
          ) : query.data.length === 0 ? (
            <EmptyState icon={icon} title={emptyTitle} description={emptyDescription} />
          ) : (
            <DataTable columns={columns} rows={query.data} rowKey={rowKey} />
          )}
        </Card>
      )}
    </>
  );
}
