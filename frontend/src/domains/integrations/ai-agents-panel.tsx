import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { cn } from "cn";
import { KeyRound, Loader2, type LucideIcon, Plug } from "lucide-react";
import { type ReactNode, useState } from "react";
import { toast } from "sonner";

import type { AccessToken } from "../../bindings/AccessToken";
import type { AccessTokenScope } from "../../bindings/AccessTokenScope";
import type { CreatedAccessToken } from "../../bindings/CreatedAccessToken";
import { Badge } from "../../components/ui/badge";
import { Button } from "../../components/ui/button";
import { Input } from "../../components/ui/input";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "../../components/ui/select";
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
import {
  claudeCodeCommand,
  mcpServersJson,
  mcpUrl,
  SCOPE_LABELS,
  splitTokens,
} from "./ai-agents";

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

/** The address every agent connects to, as the one field to copy. */
function ConnectByUrl() {
  return (
    <section aria-labelledby={`${ID}-url`} className="grid gap-2">
      <h3 id={`${ID}-url`} className="text-sm font-semibold">
        Connect by URL
      </h3>
      <div className="flex items-center gap-2 rounded-lg border bg-muted/50 py-1.5 pr-1.5 pl-3.5">
        <code className="min-w-0 flex-1 truncate text-sm">{MCP_URL}</code>
        <CopyButton
          text={MCP_URL}
          failure="The server URL could not be copied."
          variant="outline"
          size="sm"
          className="bg-card"
        >
          Copy
        </CopyButton>
      </div>
      <p className="text-sm text-muted-foreground">
        In Claude, ChatGPT or another app that signs in, add a custom connector with this
        address. You approve it here, and it is listed under Connected apps.
      </p>
    </section>
  );
}

/** One agent's access: what it is on top, what it may do and when it was last used under it. */
function TokenRow({ token }: { token: AccessToken }) {
  const queryClient = useQueryClient();
  const [confirming, setConfirming] = useState(false);
  const revoke = useMutation({
    mutationFn: () => revokeAccessToken(token.unid),
    onSuccess: async () => {
      toast.success(token.connector ? "App disconnected" : "Token revoked");
      await queryClient.invalidateQueries({ queryKey: QUERY_KEYS.accessTokens });
    },
    onError: (error) => toast.error(`It was not revoked. ${errorMessage(error)}`),
  });
  const Icon: LucideIcon = token.connector ? Plug : KeyRound;

  return (
    <li className="grid grid-cols-[auto_minmax(0,1fr)_auto] items-start gap-x-3 px-4 py-3">
      <span
        aria-hidden
        className={cn(
          "mt-0.5 flex size-8 items-center justify-center rounded-md border bg-card",
          token.connector ? "text-dusk" : "text-muted-foreground",
        )}
      >
        <Icon className="size-4" />
      </span>
      <div className="grid min-w-0 gap-1">
        <p className="font-medium break-words">{token.name}</p>
        <div className="flex flex-wrap items-center gap-x-3 gap-y-1 text-sm whitespace-nowrap text-muted-foreground">
          <Badge variant={token.scope === "read" ? "secondary" : "outline"}>
            {SCOPE_LABELS[token.scope]}
          </Badge>
          <span>
            {token.last_used_at
              ? `Used ${formatDateTime(token.last_used_at)}`
              : "Never used"}
          </span>
          <span>
            {token.connector ? "Connected" : "Made"} {formatDate(token.created_at)}
          </span>
          {token.connector ? null : (
            <span>
              Ends in <code className="text-xs">{token.last_four}</code>
            </span>
          )}
        </div>
      </div>
      <Button
        variant="ghost"
        size="sm"
        className="text-destructive hover:text-destructive"
        disabled={revoke.isPending}
        onClick={() => setConfirming(true)}
      >
        {revoke.isPending ? <Loader2 className="animate-spin" /> : null}
        {token.connector ? "Disconnect" : "Revoke"}
      </Button>
      <ConfirmDialog
        open={confirming}
        onOpenChange={setConfirming}
        title={token.connector ? `Disconnect ${token.name}?` : "Revoke this token?"}
        description={`Agents using "${token.name}" lose access to your calls at once.`}
        confirmLabel={token.connector ? "Disconnect" : "Revoke"}
        destructive
        onConfirm={() => revoke.mutate()}
      />
    </li>
  );
}

/** A titled group of tokens, one bordered list with a line between rows. */
function TokenGroup({
  id,
  title,
  tokens,
  empty,
  children,
}: {
  id: string;
  title: string;
  tokens: readonly AccessToken[];
  /** What the group is for, while it has nothing in it. */
  empty: string;
  /** Anything between the title and the list, such as the form that adds to it. */
  children?: ReactNode;
}) {
  return (
    <section aria-labelledby={id} className="grid gap-3">
      <h3 id={id} className="text-sm font-semibold">
        {title}
        {tokens.length > 0 ? (
          <span className="ml-1.5 font-normal text-muted-foreground">
            {tokens.length}
          </span>
        ) : null}
      </h3>
      {children}
      {tokens.length > 0 ? (
        <ul className="divide-y rounded-lg border">
          {tokens.map((token) => (
            <TokenRow key={token.unid} token={token} />
          ))}
        </ul>
      ) : (
        <p className="rounded-lg border border-dashed px-4 py-3 text-sm text-muted-foreground">
          {empty}
        </p>
      )}
    </section>
  );
}

/** Agents that may manage the host's calls over MCP: apps that signed in, and tokens. */
export function AiAgentsPanel() {
  const queryClient = useQueryClient();
  const tokens = useQuery(accessTokensQuery);
  const [name, setName] = useState("");
  const [scope, setScope] = useState<AccessTokenScope>("full");
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
  const { connected, personal } = splitTokens(tokens.data ?? []);

  return (
    <div className="grid gap-6">
      <ConnectByUrl />

      {tokens.isError ? (
        <p className="text-sm text-destructive">{errorMessage(tokens.error)}</p>
      ) : (
        <>
          <TokenGroup
            id={`${ID}-connected`}
            title="Connected apps"
            tokens={connected}
            empty="No app has connected yet. Add the address above in Claude or ChatGPT to see it here."
          />

          <TokenGroup
            id={`${ID}-tokens`}
            title="Tokens for other clients"
            tokens={personal}
            empty="No tokens yet. Make one for Claude Code, a script, or any client that takes a token."
          >
            {created ? (
              <NewToken created={created} onDone={() => setCreated(null)} />
            ) : (
              <form
                className="grid gap-2"
                onSubmit={(event) => {
                  event.preventDefault();
                  create.mutate({ name, scope });
                }}
              >
                <Field
                  id={`${ID}-name`}
                  label="Token name"
                  help="Where it goes, so you can tell your tokens apart. A read-only token can look at your calls and hours but not change them."
                >
                  <div className="grid gap-2 sm:grid-cols-[minmax(0,1fr)_9rem_auto]">
                    <Input
                      id={`${ID}-name`}
                      required
                      maxLength={80}
                      placeholder="Claude Code"
                      value={name}
                      aria-describedby={fieldHelpId(`${ID}-name`)}
                      onChange={(event) => setName(event.target.value)}
                      className="bg-card"
                    />
                    <div className="grid grid-cols-[minmax(0,1fr)_auto] gap-2 sm:contents">
                      <Select
                        value={scope}
                        onValueChange={(value) => setScope(value as AccessTokenScope)}
                      >
                        <SelectTrigger className="w-full bg-card" aria-label="Access">
                          <SelectValue />
                        </SelectTrigger>
                        <SelectContent>
                          {(Object.keys(SCOPE_LABELS) as AccessTokenScope[]).map(
                            (option) => (
                              <SelectItem key={option} value={option}>
                                {SCOPE_LABELS[option]}
                              </SelectItem>
                            ),
                          )}
                        </SelectContent>
                      </Select>
                      <Button type="submit" disabled={create.isPending}>
                        {create.isPending ? <Loader2 className="animate-spin" /> : null}
                        Create token
                      </Button>
                    </div>
                  </div>
                </Field>
              </form>
            )}
          </TokenGroup>
        </>
      )}
    </div>
  );
}
