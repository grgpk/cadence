import { useQuery } from "@tanstack/react-query";

import { PanelCard } from "../shared/panel-card";
import { formatDate } from "../../lib/format";
import { adminCancellationsQuery } from "./billing-api";
import { LEAVE_REASONS } from "./billing";

const OUTCOMES = {
  cancelled: "Cancelled",
  paused: "Paused",
  yearly: "Went yearly",
} as const;
const NOTES_SHOWN = 10;

/** Admin: why hosts left, or nearly left, from the cancel flow. */
export function CancellationReasons() {
  const query = useQuery(adminCancellationsQuery);
  const rows = query.data ?? [];
  if (rows.length === 0) return null;
  const notes = rows.filter((row) => row.note).slice(0, NOTES_SHOWN);
  return (
    <PanelCard
      title="Why hosts cancel"
      description={`The last ${rows.length} answers from the cancel flow.`}
    >
      <dl className="grid grid-cols-2 gap-3 sm:grid-cols-3">
        {LEAVE_REASONS.map((reason) => (
          <div key={reason.value} className="rounded-lg border px-3 py-2">
            <dt className="text-sm text-muted-foreground">{reason.label}</dt>
            <dd className="text-clock text-xl font-semibold">
              {rows.filter((row) => row.reason === reason.value).length}
            </dd>
          </div>
        ))}
      </dl>
      {notes.length > 0 ? (
        <ul className="grid gap-3">
          {notes.map((row) => (
            <li key={`${row.email}-${row.created_at}`} className="text-sm">
              <p>“{row.note}”</p>
              <p className="text-muted-foreground">
                {row.full_name}, {OUTCOMES[row.outcome].toLowerCase()} on{" "}
                {formatDate(row.created_at)}
              </p>
            </li>
          ))}
        </ul>
      ) : null}
    </PanelCard>
  );
}
