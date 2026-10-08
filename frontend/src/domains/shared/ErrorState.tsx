import { CircleAlert } from "lucide-react";

import { Alert, AlertDescription, AlertTitle } from "../../components/ui/alert";
import { Button } from "../../components/ui/button";

export function ErrorState({
  title,
  error,
  onRetry,
}: {
  title: string;
  error: unknown;
  onRetry?: () => void;
}) {
  return (
    <Alert variant="destructive">
      <CircleAlert className="size-4" />
      <AlertTitle>{title}</AlertTitle>
      <AlertDescription>
        <p>{error instanceof Error ? error.message : "Something went wrong."}</p>
        {onRetry ? (
          <Button variant="outline" size="sm" className="mt-2" onClick={onRetry}>
            Try again
          </Button>
        ) : null}
      </AlertDescription>
    </Alert>
  );
}
