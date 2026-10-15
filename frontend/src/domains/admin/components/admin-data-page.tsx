import { useQuery } from "@tanstack/react-query";
import type { LucideIcon } from "lucide-react";
import { Search } from "lucide-react";
import { useState } from "react";

import { Card } from "../../../components/ui/card";
import { Input } from "../../../components/ui/input";
import { DataTable, type Column } from "../../shared/data-table";
import { EmptyState } from "../../shared/empty-state";
import { ErrorState } from "../../shared/error-state";
import { PageHeader } from "../../shared/page-header";
import { TableSkeleton } from "../../shared/table-skeleton";
import { adminDataQuery } from "../routes";

type AdminDataPageProps<T> = {
  title: string;
  description: string;
  icon: LucideIcon;
  endpoint: string;
  emptyTitle: string;
  emptyDescription: string;
  rowKey: (item: T) => string;
  columns: Column<T>[];
  /** Everything about a row someone might type to find it. Turns the search box on. */
  searchText?: (item: T) => string;
  /** What the search box starts with, for links that open the list already narrowed. */
  initialSearch?: string;
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
  searchText,
  initialSearch = "",
}: AdminDataPageProps<T>) {
  const [search, setSearch] = useState(initialSearch);
  const query = useQuery(adminDataQuery<T[]>(endpoint));

  const all = query.data ?? [];
  const wanted = search.trim().toLowerCase();
  const rows =
    searchText && wanted
      ? all.filter((row) => searchText(row).toLowerCase().includes(wanted))
      : all;

  return (
    <>
      <PageHeader
        title={title}
        description={description}
        actions={
          <>
            {query.data ? (
              <p className="text-sm text-muted-foreground tabular-nums">
                {rows.length === all.length ? "" : `${rows.length} of `}
                {all.length} {all.length === 1 ? "record" : "records"}
              </p>
            ) : null}
            {searchText ? (
              <div className="relative w-full sm:w-auto">
                <Search className="pointer-events-none absolute top-1/2 left-2.5 size-4 -translate-y-1/2 text-muted-foreground" />
                <Input
                  type="search"
                  aria-label={`Search ${title.toLowerCase()}`}
                  placeholder="Search"
                  value={search}
                  onChange={(event) => setSearch(event.target.value)}
                  className="w-full bg-card pl-8 sm:w-56"
                />
              </div>
            ) : null}
          </>
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
          ) : all.length === 0 ? (
            <EmptyState icon={icon} title={emptyTitle} description={emptyDescription} />
          ) : rows.length === 0 ? (
            <EmptyState
              icon={Search}
              title={`Nothing matches "${search.trim()}"`}
              description="Check the spelling, or clear the search to see everything."
            />
          ) : (
            <DataTable columns={columns} rows={rows} rowKey={rowKey} />
          )}
        </Card>
      )}
    </>
  );
}
