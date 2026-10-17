import { Avatar, AvatarFallback } from "../../components/ui/avatar";

/**
 * Who the call is with. The same element on the pick-a-call page and the
 * booking card, so it carries over when the visitor picks a call.
 */
export function HostBadge({ name }: { name: string }) {
  return (
    <div className="vt-host flex items-center gap-2.5">
      <Avatar aria-hidden="true">
        <AvatarFallback className="bg-primary font-display font-semibold text-primary-foreground">
          {name.trim().charAt(0).toUpperCase()}
        </AvatarFallback>
      </Avatar>
      <p className="min-w-0 text-sm font-medium break-words text-muted-foreground">
        {name}
      </p>
    </div>
  );
}
