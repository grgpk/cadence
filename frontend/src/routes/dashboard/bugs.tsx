import { createFileRoute } from "@tanstack/react-router";
import { useEffect, useState } from "react";
import type { AdminBugReport } from "../../bindings/AdminBugReport";
import { api } from "../../lib/api";

export const Route = createFileRoute("/dashboard/bugs")({ component: BugsPage });
function BugsPage() {
  const [bugs, setBugs] = useState<AdminBugReport[]>([]);
  useEffect(() => {
    api<AdminBugReport[]>("/api/admin/bug-reports")
      .then(setBugs)
      .catch(() => setBugs([]));
  }, []);
  return (
    <section>
      <h1>Bug reports</h1>
      <div className="card">
        {bugs.length === 0 ? (
          <p>No bug reports.</p>
        ) : (
          bugs.map((bug) => (
            <article
              key={bug.unid}
              style={{ borderBottom: "1px solid #eee", padding: "12px 0" }}
            >
              <strong>{bug.bugtype}</strong>
              <p>{bug.message}</p>
              <small>{new Date(bug.created).toLocaleString()}</small>
            </article>
          ))
        )}
      </div>
    </section>
  );
}
