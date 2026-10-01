# SOP: improve a `PLAN_N` before it is implemented

Plan exists, no code written yet. Cost of change is near zero now and high
later, so scrutinise hard. But do not grow the phase: improvements that add
scope go to "Out of scope" as named follow-ups.

## Do

1. Re-read the whole plan, then the principles in `PLAN_0_GENERAL.md`.
2. Read the code it targets, and `Cargo.lock` / `pnpm-lock.yaml` for the
   exact version of every crate or package it leans on (`axum`, `sqlx`,
   `time`, `time-tz`, `axum_session`, `ts-rs`, `next`, ...). New versions add
   helpers the plan may be hand-rolling.
3. Sweep the official sources of those crates (axum `examples/`, sqlx,
   docs.rs) for the same pattern with the **same versions**. Get `path:line`.
4. Walk the plan's approach against those refs, one step at a time. Flag:
   - hand-rolled code where the crate already has it (`time-tz` vs manual
     offsets, axum extractors and rejections vs custom parsing).
   - layer leaks: SQL outside `db.rs`, rules in `handlers.rs`, a domain
     calling another domain's handlers, admin code outside the domain's
     `admin_handlers.rs`.
   - a route without a named guard, an admin handler without `AdminUser`.
   - wire types written by hand on the frontend instead of `bindings/`.
   - side effects (mail) done inside a request instead of via `email_queue`.
   - time handling: naive date times crossing a function boundary, UTC
     assumed where a zone is needed, DST gaps and overlaps ignored.
   - error mapping that leaks internals (`e.to_string()` of a sqlx error
     sent to the client).
   - from phase 9: queries on host data missing `WHERE host_id = $n`.
5. Fold in the cheap, no-behaviour-change fixes. Anything that changes
   behaviour or touches other phases: move to "Out of scope" with the reason
   and where it lands.
6. Propagate. A changed name, column or route must update every dependent
   section: data model, API table, flow schema, impl sketch, tests,
   milestones, definition of done, "After phase N".

## Keep

- The plan's structure, voice, and ASCII schemas. Edit in place, do not
  rewrite.
- No em dashes, no `---` rules.
- Every new claim about a reference carries `path:line`.

## Ship

- Commit straight to `main`: `docs(plan): PLAN_<N> <what changed>`.
