import { cva, type VariantProps } from "class-variance-authority";
import type * as React from "react";
import { cn } from "../../lib/utils";
const badgeVariants = cva("inline-flex items-center rounded-full border px-2.5 py-0.5 text-xs font-semibold", { variants: { variant: { default: "border-transparent bg-neutral-950 text-white", secondary: "border-transparent bg-neutral-100 text-neutral-900", outline: "text-neutral-950" } }, defaultVariants: { variant: "default" } });
export function Badge({ className, variant, ...props }: React.HTMLAttributes<HTMLDivElement> & VariantProps<typeof badgeVariants>) { return <div className={cn(badgeVariants({ variant }), className)} {...props} />; }
