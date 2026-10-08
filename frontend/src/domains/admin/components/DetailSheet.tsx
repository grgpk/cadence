import type { ReactNode } from "react";

import {
  Sheet,
  SheetContent,
  SheetDescription,
  SheetHeader,
  SheetTitle,
  SheetTrigger,
} from "../../../components/ui/sheet";

type DetailField = { label: string; value: ReactNode };

/** A row's name as a button that opens everything the table has no room for. */
export function DetailSheet({
  trigger,
  title,
  description,
  fields,
  children,
}: {
  trigger: string;
  title: string;
  description: string;
  fields: DetailField[];
  children?: ReactNode;
}) {
  return (
    <Sheet>
      <SheetTrigger className="max-w-full truncate rounded-sm text-left font-medium underline-offset-4 outline-none hover:underline focus-visible:ring-[3px] focus-visible:ring-ring/50">
        {trigger}
      </SheetTrigger>
      <SheetContent className="w-full gap-0 overflow-y-auto sm:max-w-lg">
        <SheetHeader className="border-b">
          <SheetTitle className="pr-8 break-words">{title}</SheetTitle>
          <SheetDescription>{description}</SheetDescription>
        </SheetHeader>
        <dl className="divide-y text-sm">
          {fields.map((field) => (
            <div key={field.label} className="grid gap-1 px-4 py-3">
              <dt className="text-muted-foreground">{field.label}</dt>
              <dd className="break-words whitespace-pre-wrap">
                {field.value ?? (
                  <span className="text-muted-foreground">Not recorded</span>
                )}
              </dd>
            </div>
          ))}
        </dl>
        {children}
      </SheetContent>
    </Sheet>
  );
}
