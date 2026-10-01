# Plan 5: test hardening

Status: planned.

## Goal

A contributor can prove auth, ownership, booking conflicts, migrations, and
admin pages without real Google or email providers.

## The gap this fixes

`backend/tests/export_bindings.rs` exists, but domain HTTP coverage is small.
Frontend build and static checks pass, but there is no complete browser smoke
path for both sidebars and populated seed data.

## Approach

- Add one HTTP integration test file per domain using the real router and
  `#[sqlx::test(migrations = "../migrations")]`.
- Use deterministic fakes for calendar and SMTP.
- Test UI route access and loading/error/empty/populated states, not shadcn internals.
- Keep `request.http` synchronized with route changes.

## API

No production routes. Test routes from Plans 2, 3, and 4.

## Flow schema

```
fixture DB ─▶ real router ─▶ cookie/token request
                         ├─ status and JSON
                         ├─ DB side effects
                         └─ isolation and retry assertions
```

## Tests

- Auth matrix: public, logged out, Host, Admin, Root.
- Host A cannot read Host B.
- Booking valid, invalid, conflict, queue atomicity.
- Bug report public write and admin aggregation.
- Fresh migration and seed verification.
- `/dashboard` and `/admin` sidebar smoke flow.

## Milestones

- T0: HTTP test harness.
- T1: auth and ownership matrix.
- T2: booking and queue contracts.
- T3: frontend smoke flow.
- T4: CI quality job.

## Definition of done

- Every protected endpoint has 401 and authorization coverage.
- Every host query has an isolation test.
- Tests use fake providers.
- CI runs from a clean checkout.

## After phase 5

Phase 6 packages the verified app for deployment and embed use.
