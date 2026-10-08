import { CircleAlert } from "lucide-react";

import { Alert, AlertDescription, AlertTitle } from "../../components/ui/alert";
import { Button } from "../../components/ui/button";
import { errorMessage } from "../../lib/error-message";

export function ErrorState({
  title,
  error,
  description,
  onRetry,
}: {
  title: string;
  error: unknown;
  /** Says what happened when there is no failed request to explain it. */
  description?: string;
  onRetry?: () => void;
}) {
  return (
    <Alert variant="destructive">
      <CircleAlert className="size-4" />
      <AlertTitle>{title}</AlertTitle>
      <AlertDescription>
        <p>{description ?? errorMessage(error)}</p>
        {onRetry ? (
          <Button variant="outline" size="sm" className="mt-2" onClick={onRetry}>
            Try again
          </Button>
        ) : null}
      </AlertDescription>
    </Alert>
  );
}
