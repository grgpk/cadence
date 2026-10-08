import { useMutation } from "@tanstack/react-query";
import { Link, useNavigate, type LinkProps } from "@tanstack/react-router";
import { Loader2 } from "lucide-react";
import { useEffect, useRef } from "react";

import { Button } from "../../components/ui/button";
import { submitGoogleCalendarCode } from "../dashboard/host-api";
import { ErrorState } from "./ErrorState";

/** Finishes the Google sign-in round trip, then returns to the page that started it. */
export function GoogleCalendarCallback({
  code,
  returnTo,
}: {
  code: string;
  returnTo: LinkProps["to"] & string;
}) {
  const navigate = useNavigate();
  const submitted = useRef(false);
  const connect = useMutation({
    mutationFn: submitGoogleCalendarCode,
    onSuccess: () => navigate({ to: returnTo }),
  });

  useEffect(() => {
    if (!code || submitted.current) return;
    submitted.current = true;
    connect.mutate(code);
  }, [code, connect]);

  return (
    <section className="mx-auto flex w-full max-w-md flex-1 flex-col justify-center gap-4">
      {connect.error || !code ? (
        <>
          <ErrorState
            title="Google Calendar was not connected"
            error={connect.error}
            description={code ? undefined : "Google did not return a code."}
          />
          <Button variant="outline" className="self-start" asChild>
            <Link to={returnTo}>Go back</Link>
          </Button>
        </>
      ) : (
        <p className="flex items-center justify-center gap-2 text-sm text-muted-foreground">
          <Loader2 className="size-4 animate-spin" />
          Connecting Google Calendar
        </p>
      )}
    </section>
  );
}
