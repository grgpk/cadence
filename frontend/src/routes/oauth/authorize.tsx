import { useMutation, useQuery } from "@tanstack/react-query";
import { createFileRoute } from "@tanstack/react-router";
import { Loader2 } from "lucide-react";
import { useState } from "react";

import type { AccessTokenScope } from "../../bindings/AccessTokenScope";
import { Button } from "../../components/ui/button";
import { Label } from "../../components/ui/label";
import { RadioGroup, RadioGroupItem } from "../../components/ui/radio-group";
import { SCOPE_LABELS } from "../../domains/integrations/ai-agents";
import {
  authorize,
  oauthClientQuery,
  oauthParam,
  redirectHost,
} from "../../domains/integrations/oauth-api";
import { AuthCard } from "../../domains/shared/auth-card";
import { RouteError } from "../../domains/shared/not-found";
import { APP_NAME, NOINDEX_META } from "../../lib/config";

const ID = "oauth-scope";

const SCOPE_HELP: Record<AccessTokenScope, string> = {
  full: "Read, book, move and cancel your calls, and change your hours and kinds of call.",
  read: "Read your calls, hours and kinds of call. It cannot change anything.",
};

export const Route = createFileRoute("/oauth/authorize")({
  validateSearch: (search) => ({
    client_id: oauthParam(search.client_id),
    redirect_uri: oauthParam(search.redirect_uri),
    code_challenge: oauthParam(search.code_challenge),
    code_challenge_method: oauthParam(search.code_challenge_method),
    state: oauthParam(search.state),
    scope: oauthParam(search.scope),
  }),
  head: () => ({
    meta: [{ title: `Connect an app | ${APP_NAME}` }, NOINDEX_META],
  }),
  component: AuthorizePage,
  errorComponent: RouteError,
});

function AuthorizePage() {
  const search = Route.useSearch();
  const { user } = Route.useRouteContext();
  const [scope, setScope] = useState<AccessTokenScope>(
    search.scope === "read" ? "read" : "full",
  );
  const client = useQuery({
    ...oauthClientQuery(search.client_id ?? ""),
    enabled: Boolean(search.client_id),
  });
  const answer = useMutation({
    mutationFn: authorize,
    onSuccess: ({ url }) => window.location.assign(url),
  });

  const { client_id, redirect_uri, code_challenge, code_challenge_method } = search;
  const known =
    client.data && redirect_uri
      ? client.data.redirect_uris.includes(redirect_uri)
      : false;
  if (!client_id || !redirect_uri || !code_challenge || !code_challenge_method) {
    return <BadLink />;
  }
  if (client.isPending) return null;
  if (!client.data || !known) return <BadLink />;

  function reply(approve: boolean) {
    if (!client_id || !redirect_uri || !code_challenge || !code_challenge_method) return;
    answer.mutate({
      client_id,
      redirect_uri,
      code_challenge,
      code_challenge_method,
      state: search.state,
      scope,
      approve,
    });
  }

  return (
    <AuthCard
      title={`Connect ${client.data.client_name}`}
      description={`It will act for you in ${APP_NAME}, signed in as ${user?.email ?? "you"}.`}
      error={answer.error}
      onSubmit={(event) => {
        event.preventDefault();
        reply(true);
      }}
      footer={`You go back to ${redirectHost(redirect_uri)} after this. You can disconnect it any time in Apps.`}
    >
      <RadioGroup
        value={scope}
        onValueChange={(value) => setScope(value as AccessTokenScope)}
        aria-label="What it may do"
        className="grid gap-3"
      >
        {(Object.keys(SCOPE_LABELS) as AccessTokenScope[]).map((option) => (
          <Label
            key={option}
            htmlFor={`${ID}-${option}`}
            className="flex cursor-pointer items-start gap-3 rounded-lg border p-4 font-normal hover:bg-muted/40 has-[[data-state=checked]]:border-foreground has-[[data-state=checked]]:bg-muted/40"
          >
            <span className="grid min-w-0 flex-1 gap-1">
              <span className="font-medium">{SCOPE_LABELS[option]}</span>
              <span className="text-sm text-muted-foreground">{SCOPE_HELP[option]}</span>
            </span>
            <RadioGroupItem id={`${ID}-${option}`} value={option} className="mt-0.5" />
          </Label>
        ))}
      </RadioGroup>
      <div className="flex justify-end gap-2">
        <Button
          type="button"
          variant="outline"
          disabled={answer.isPending}
          onClick={() => reply(false)}
        >
          Cancel
        </Button>
        <Button type="submit" disabled={answer.isPending}>
          {answer.isPending ? <Loader2 className="animate-spin" /> : null}
          Allow
        </Button>
      </div>
    </AuthCard>
  );
}

/** A link that is missing parts, or names an app or address we do not know. */
function BadLink() {
  return (
    <AuthCard
      title="This link does not work"
      description="Start connecting again from the app you came from."
      error={null}
      onSubmit={(event) => event.preventDefault()}
      footer={null}
    >
      {null}
    </AuthCard>
  );
}
