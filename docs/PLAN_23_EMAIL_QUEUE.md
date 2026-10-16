# Plan 23: the email queue owns booking emails

Status: implemented. Backend fmt and tests pass (100 unit, 15 auth, 40
booking HTTP); bindings unchanged; `.sqlx` regenerated. Clippy adds only doc
and `must_use` findings of the kind the code already has (see Plan 22).

From the architecture review (candidate 3). Terms are in `CONTEXT.md`.

## Goal

Which emails a call sends, to whom and when is decided in one place, and the
row that is written is the row that is read back.

## The gap this fixes

- Bookings picked the templates, timed the 24h, 2h and 30 minute reminders,
  built both payloads with `json!`, and wrote and deleted `email_queue` rows
  itself. The email module wrote the same table for password resets.
- `render` read payload keys by string that another module had written.

## Approach

No new dependency, migration or wire type.

```
bookings::service ──(in its transaction)──▶ emails::booked / rescheduled / cancelled
                                               │ templates, reminder times, payloads
                                               ▼
                                            emails::db (only writer of email_queue)
                                               ▼
                                   worker ──▶ delivery ──▶ render (same payload types)
```

- `emails::service::{booked, rescheduled, cancelled}` take the booking, its
  page and `now`; `cancelled` also takes `CancelledBy`. A move or a cancel
  first drops what still waits. Reminders are queued only while ahead.
- `InviteeEmail`, `HostEmail` and `ResetEmail` write the payload and read it
  back. The keys are unchanged; every field is optional and read on its own,
  so rows already queued, or with a value that no longer reads, still render.
- `emails::db::queue` takes an optional booking and a send time;
  `emails::db::drop_unsent` drops a call's waiting emails. The copies in
  `bookings::db` are gone.

## Tests

- Through the email queue's own functions (`#[sqlx::test]`): booked 3 hours
  ahead skips the 24 hour reminder; a move replaces what waited and times the
  reminders from the new start; a cancel by the host tells the invitee and by
  the invitee tells the host, with nothing else left.
- Unit: `delivery` drop rules (cancelled call, started call, no call, unknown
  template); a mistyped or non-object payload still renders; a written
  payload reads back as written.
- HTTP slimmed: the manage-link and move tests check only that both sides were
  told; the drop test keeps the cancelled case, which needs the worker's join.

## Definition of done

- [x] Bookings tells the email queue what happened and nothing more.
- [x] One writer of `email_queue`.
- [x] Payloads are typed; old rows still render.
