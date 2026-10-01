# SOP: create the next `docs/PLAN_N_*.md`

Run this every time a new phase plan is needed. One phase = one
`PLAN_N_<TOPIC>.md` = one mergeable slice. Keep it small, referenced, and
testable. Copy the shape of the previous `PLAN_N`, it is the template.

`N` = the phase number in the `PLAN_0_GENERAL.md` table. Filename
`PLAN_<N>_<SHORT_TOPIC>.md`, all caps topic (`PLAN_4_CALENDAR_BOOKING.md`).
If it slots between existing phases, renumber the later ones in a separate
commit (`docs(plan): renumber ...`). Flip its row in `PLAN_0` from 👉 to 📅.

## Before writing

1. Read `PLAN_0_GENERAL.md`, the previous phase plan, and the code the phase
   touches (routes, handlers, `db.rs`, migrations, frontend domain).
2. Find the pattern in the official sources of the crates it leans on
   (axum `examples/`, sqlx, `axum_session`, `ts-rs`, Google Calendar API
   docs). Note exact `path:line` or URL.
3. Decide the single smallest change a visitor, host or admin can see. Push
   everything else to "Out of scope".

## Required sections, in order

- **Goal**: one paragraph, the user-facing behaviour, from the visitor's,
  host's or admin's point of view.
- **The gap this fixes**: the current code, quoted with `path:line`, and the
  line where it stops short. First phase: what does not exist yet.
- **Approach**: the reference implementation(s), quoted or paraphrased
  tight, with `path:line`. State which parts cadence keeps and which it
  drops, each with a reason.
- **Data model**: the migration(s) as SQL, new columns and constraints, and
  what happens to existing rows (backfill, default, `NOT NULL` order).
- **API**: every route added or changed as a table (method, path, guard:
  public / logged in / `AdminUser`, request, response, error codes), and the
  `app/api/.../route.ts` proxy that fronts it.
- **Flow schema**: ASCII sequence or decision tree for the core logic
  (request to response, or click to API call to screen). This is the spec,
  code sketches follow it. Frontend phases add an ASCII screen schema.
- **Impl sketch**: real code, real function names, one block per layer
  (`routes.rs`, `handlers.rs` / `admin_handlers.rs`, `service.rs`, `db.rs`,
  `models.rs`, frontend `api.ts` + component), `// step N` comments tying
  back to the flow schema. Pure logic (time zone conversion) as small `fn`s
  with no I/O.
- **Out of scope**: bulleted, bold lead, each with where it lands later
  (phase number) and why it is safe to defer.
- **Tests**: `backend/tests/<domain>_http.rs`, one bullet per case. Cover:
  happy path, logged out (401), not admin (403), invalid input, conflict,
  boundary (midnight, DST change, last slot of the day), prior phase tests
  still green. New wire types added to `tests/export_bindings.rs`.
- **Milestones**: `X0..Xn` (letter = phase initial). X0 = migration and
  types, no behaviour change, existing tests green. Middle = behaviour and
  its tests. Last = clippy clean, `.sqlx/` and `bindings/` regenerated,
  frontend `lint` + `typecheck` green, all prior milestones green.
- **Definition of done**: flat checklist a reviewer runs. Every "rejects"
  and "does nothing" case listed explicitly. Commands that must pass.
- **After phase N**: the next plan, and the hook this one leaves for it.

## Rules

- ASCII schemas for anything with a flow, a branch or a screen. Cheaper to
  read than prose.
- Every claim about a reference carries `path:line`.
- Quote real cadence identifiers (functions, tables, columns, routes), not
  invented ones. Grep to confirm they exist. New names are marked as new.
- Every backend piece follows the domain shape from `PLAN_0_GENERAL.md`
  (`routes`, `handlers`, `service`, `models`, `db`, admin views in that
  domain's `admin_handlers.rs`). Every wire type derives `TS` and lands in
  `frontend/src/bindings/`. Frontend code goes in `src/domain/<same name>/`.
- No em dashes, no `---` rules. Commas, colons, parens.
- A phase never changes a route's JSON shape silently: a changed shape is
  listed in the API table with the old one.
- Security first: every route names its guard. From phase 9, every query on
  host data filters by `host_id`.

## Ship

- Commit straight to `main`: `docs(plan): add PLAN_<N> <topic>`. No branch,
  no PR.
