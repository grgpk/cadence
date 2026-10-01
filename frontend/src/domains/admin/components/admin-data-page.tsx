import { useEffect, useState, type ReactNode } from "react";

import { Badge } from "../../../components/ui/badge";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "../../../components/ui/card";
import { api } from "../../../lib/api";

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
  const [items, setItems] = useState<T[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    let active = true;
    void api<T[]>(endpoint)
      .then((result) => {
        if (active) {
          setItems(result);
          setLoading(false);
        }
      })
      .catch((requestError: unknown) => {
        if (active) {
          setError(
            requestError instanceof Error ? requestError.message : "Request failed",
          );
          setLoading(false);
        }
      });

    return () => {
      active = false;
    };
  }, [endpoint]);

  return (
    <section className="flex flex-1 flex-col gap-6 p-6">
      <header>
        <h1 className="text-2xl font-semibold tracking-tight">{title}</h1>
        <p className="text-muted-foreground">{description}</p>
      </header>
      <Card>
        <CardHeader>
          <CardTitle>{loading ? "Loading..." : `${items.length} records`}</CardTitle>
          <CardDescription>{error ?? emptyMessage}</CardDescription>
        </CardHeader>
        <CardContent>
          {error ? (
            <Badge variant="outline">{error}</Badge>
          ) : items.length === 0 && !loading ? (
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
