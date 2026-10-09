import { cn } from "cn";
import type { ReactNode } from "react";

import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "../../components/ui/table";

export type Column<T> = {
  header: string;
  cell: (row: T) => ReactNode;
  className?: string;
};

export function DataTable<T>({
  columns,
  rows,
  rowKey,
}: {
  columns: Column<T>[];
  rows: T[];
  rowKey: (row: T) => string;
}) {
  const [title, ...details] = columns;
  return (
    <>
      {/* A phone has no room for columns: each row becomes its own short list. */}
      <ul className="divide-y sm:hidden">
        {rows.map((row) => (
          <li key={rowKey(row)} className="grid gap-2 px-5 py-3.5 text-sm">
            {title ? <div className="min-w-0">{title.cell(row)}</div> : null}
            <dl className="grid gap-1.5">
              {details.map((column) => (
                <div
                  key={column.header}
                  className="flex items-center justify-between gap-4"
                >
                  <dt className="shrink-0 text-xs text-muted-foreground">
                    {column.header}
                  </dt>
                  <dd className="min-w-0 text-right">{column.cell(row)}</dd>
                </div>
              ))}
            </dl>
          </li>
        ))}
      </ul>
      <div className="hidden sm:block">
        <DesktopTable columns={columns} rows={rows} rowKey={rowKey} />
      </div>
    </>
  );
}

function DesktopTable<T>({
  columns,
  rows,
  rowKey,
}: {
  columns: Column<T>[];
  rows: T[];
  rowKey: (row: T) => string;
}) {
  return (
    <Table>
      <TableHeader>
        <TableRow className="hover:bg-transparent">
          {columns.map((column) => (
            <TableHead
              key={column.header}
              className={cn(
                "px-4 text-xs text-muted-foreground first:pl-5 last:pr-5",
                column.className,
              )}
            >
              {column.header}
            </TableHead>
          ))}
        </TableRow>
      </TableHeader>
      <TableBody>
        {rows.map((row) => (
          <TableRow key={rowKey(row)}>
            {columns.map((column) => (
              <TableCell
                key={column.header}
                className={cn("px-4 py-3 first:pl-5 last:pr-5", column.className)}
              >
                {column.cell(row)}
              </TableCell>
            ))}
          </TableRow>
        ))}
      </TableBody>
    </Table>
  );
}

/** Two stacked lines in one cell: the thing, and a quieter detail under it. */
export function TwoLine({
  primary,
  secondary,
}: {
  primary: ReactNode;
  secondary: ReactNode;
}) {
  return (
    <div className="min-w-0">
      <p className="truncate font-medium">{primary}</p>
      <p className="truncate text-muted-foreground">{secondary}</p>
    </div>
  );
}
