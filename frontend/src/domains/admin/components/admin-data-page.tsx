import { useQuery } from "@tanstack/react-query";
import type { ReactNode } from "react";

import { Badge } from "../../../components/ui/badge";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "../../../components/ui/card";
import { api } from "../../../lib/api";
import { QUERY_KEYS } from "../../../lib/query-keys";

type AdminDataPageProps<T> = {
  title: string;
  description: string;
  endpoint: string;
  emptyMessage: string;
  itemKey: (item: T) => string;
  renderItem: (item: T) => ReactNode;
};

export function AdminDataPage<T>({
  title,
  description,
  endpoint,
  emptyMessage,
  itemKey,
  renderItem,
}: AdminDataPageProps<T>) {
  const {
    data: items = [],
    error,
    isPending,
  } = useQuery({
    queryKey: QUERY_KEYS.adminData(endpoint),
    queryFn: () => api<T[]>(endpoint),
  });
  const errorMessage = error instanceof Error ? error.message : null;

  return (
    <section className="flex flex-1 flex-col gap-6 p-6">
      <header>
        <h1 className="text-2xl font-semibold tracking-tight">{title}</h1>
        <p className="text-muted-foreground">{description}</p>
      </header>
      <Card>
        <CardHeader>
          <CardTitle>{isPending ? "Loading..." : `${items.length} records`}</CardTitle>
          <CardDescription>{errorMessage ?? emptyMessage}</CardDescription>
        </CardHeader>
        <CardContent>
          {errorMessage ? (
            <Badge variant="outline">{errorMessage}</Badge>
          ) : items.length === 0 && !isPending ? (
            <p className="text-sm text-muted-foreground">{emptyMessage}</p>
          ) : (
            <div className="space-y-3">
              {items.map((item) => (
                <div key={itemKey(item)}>{renderItem(item)}</div>
              ))}
            </div>
          )}
        </CardContent>
      </Card>
    </section>
  );
}

export function formatAdminDate(value: string) {
  return new Intl.DateTimeFormat("en", {
    dateStyle: "medium",
    timeStyle: "short",
  }).format(new Date(value));
}
