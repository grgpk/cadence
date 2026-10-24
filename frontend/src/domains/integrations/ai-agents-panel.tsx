import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { KeyRound, Loader2, Trash2 } from "lucide-react";
import { useState } from "react";
import { toast } from "sonner";

import type { AccessToken } from "../../bindings/AccessToken";
import type { CreatedAccessToken } from "../../bindings/CreatedAccessToken";
import { Button } from "../../components/ui/button";
import { Input } from "../../components/ui/input";
import { API_URL } from "../../lib/config";
import { errorMessage } from "../../lib/error-message";
import { formatDate, formatDateTime } from "../../lib/format";
import { QUERY_KEYS } from "../../lib/query-keys";
import { ConfirmDialog } from "../shared/confirm-dialog";
import { CopyButton } from "../shared/copy-button";
import { Field, fieldHelpId } from "../shared/field";
import {
  accessTokensQuery,
  createAccessToken,
  revokeAccessToken,
} from "./access-tokens-api";
import { claudeCodeCommand, mcpServersJson, mcpUrl } from "./ai-agents";

const ID = "access-token";
const CREATE_ERRORS = {
  400: "Give it a name of up to 80 characters.",
  409: "You have 20 tokens, the most there can be. Revoke one first.",
};

const MCP_URL = mcpUrl(API_URL);

/** Text to copy as it is, with a button that copies it and ticks once it has. */
function Copyable({ label, text }: { label: string; text: string }) {
  return (
    <div className="grid gap-1.5">
      <div className="flex items-center justify-between gap-2">
        <span className="text-sm font-medium">{label}</span>
        <CopyButton
          text={text}
          failure={`The ${label.toLowerCase()} could not be copied.`}
          variant="ghost"
          size="icon"
          className="size-7"
          aria-label={`Copy ${label.toLowerCase()}`}
        />
      </div>
      <pre className="overflow-x-auto rounded-md bg-muted px-3 py-2 text-xs whitespace-pre">
        {text}
      </pre>
    </div>
  );
}

/** The new token's only showing, with the setup that uses it. */
function NewToken({
  created,
  onDone,
}: {
  created: CreatedAccessToken;
  onDone: () => void;
}) {
  return (
    <div className="grid gap-4 rounded-lg border border-primary/40 p-4">
      <div>
        <p className="font-medium">Copy your token now</p>
        <p className="mt-1 text-sm text-muted-foreground">
          It is shown once. Anyone with it can manage your calls, so keep it like a
          password.
        </p>
      </div>
      <Copyable label="Token" text={created.token} />
      <Copyable label="Claude Code" text={claudeCodeCommand(MCP_URL, created.token)} />
      <Copyable label="Other MCP clients" text={mcpServersJson(MCP_URL, created.token)} />
      <Button variant="outline" size="sm" className="w-fit" onClick={onDone}>
        I saved it
      </Button>
    </div>
  );
}

function TokenRow({ token }: { token: AccessToken }) {
  const queryClient = useQueryClient();
  const [confirming, setConfirming] = useState(false);
  const revoke = useMutation({
    mutationFn: () => revokeAccessToken(token.unid),
    onSuccess: async () => {
      toast.success("Token revoked");
      await queryClient.invalidateQueries({ queryKey: QUERY_KEYS.accessTokens });
    },
    onError: (error) => toast.error(`It was not revoked. ${errorMessage(error)}`),
  });

  return (
    <li className="flex flex-wrap items-center gap-x-4 gap-y-1 rounded-lg border p-3 text-sm">
      <KeyRound className="size-4 text-muted-foreground" />
      <span className="min-w-0 flex-1 truncate font-medium">{token.name}</span>
      <code className="text-xs text-muted-foreground">…{token.last_four}</code>
      <span className="text-muted-foreground">
        {token.last_used_at ? `Used ${formatDateTime(token.last_used_at)}` : "Never used"}
      </span>
      <span className="text-muted-foreground">Made {formatDate(token.created_at)}</span>
      <Button
        variant="outline"
        size="sm"
        className="text-destructive"
        disabled={revoke.isPending}
        onClick={() => setConfirming(true)}
      >
        <Trash2 />
        Revoke
      </Button>
      <ConfirmDialog
        open={confirming}
        onOpenChange={setConfirming}
        title="Revoke this token?"
        description={`Agents using "${token.name}" lose access to your calls at once.`}
        confirmLabel="Revoke"
        destructive
        onConfirm={() => revoke.mutate()}
      />
    </li>
  );
}

/** Tokens that let an AI agent manage the host's calls over MCP. */
export function AiAgentsPanel() {
  const queryClient = useQueryClient();
  const tokens = useQuery(accessTokensQuery);
  const [name, setName] = useState("");
  const [created, setCreated] = useState<CreatedAccessToken | null>(null);

  const create = useMutation({
    mutationFn: createAccessToken,
    onSuccess: async (next) => {
      setName("");
      setCreated(next);
      await queryClient.invalidateQueries({ queryKey: QUERY_KEYS.accessTokens });
    },
    onError: (error) =>
      toast.error(`No token was made. ${errorMessage(error, CREATE_ERRORS)}`),
  });

  return (
    <div className="grid gap-5">
      <Copyable label="Server URL" text={MCP_URL} />

      {created ? (
        <NewToken created={created} onDone={() => setCreated(null)} />
      ) : (
        <form
          className="grid gap-2"
          onSubmit={(event) => {
            event.preventDefault();
            create.mutate(name);
          }}
        >
          <Field
            id={`${ID}-name`}
            label="Token name"
            help="Where it goes, so you can tell your tokens apart."
          >
            <div className="flex flex-wrap gap-2">
              <Input
                id={`${ID}-name`}
                required
                maxLength={80}
                placeholder="Claude Code"
                value={name}
                aria-describedby={fieldHelpId(`${ID}-name`)}
                onChange={(event) => setName(event.target.value)}
                className="min-w-0 flex-1 bg-card"
              />
              <Button type="submit" disabled={create.isPending}>
                {create.isPending ? <Loader2 className="animate-spin" /> : null}
                Create token
              </Button>
            </div>
          </Field>
        </form>
      )}

      {tokens.data && tokens.data.length > 0 ? (
        <ul className="grid gap-2">
          {tokens.data.map((token) => (
            <TokenRow key={token.unid} token={token} />
          ))}
        </ul>
      ) : tokens.isError ? (
        <p className="text-sm text-destructive">{errorMessage(tokens.error)}</p>
      ) : null}
    </div>
  );
}
