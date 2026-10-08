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
