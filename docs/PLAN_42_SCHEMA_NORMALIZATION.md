# Plan 42: one place for each fact in the schema

Status: implemented.

## Goal

Every table is created once, in its final shape, and every fact lives in one
row: a person, a question, an answer, a visit. Migrations hold no `ALTER`.

## The gap this fixes

- `0018` and `0020` altered `bookings`, `leads` and `widget_settings`, so no
  CREATE showed a table as it is. `bookings.event_type_unid` pointed at a
  table created nine files later.
- A person was stored twice: `bookings.invitee_name/invitee_email` and the
  name, email and phone on `leads`. They were matched by `lower(email)` text.
- `leads` copied the visit's page, UTM tags, IP and user agent. IP and user
  agent were never written; `bookings.calling_visit_unid` was never read.
- Five `ask_*` booleans on `event_types` mirrored five answer columns on
  `leads`, and `leads.qualification_status` stored what the investment
  answer already says.
- `bug_reports` had both `id` and `unid` and run-together names (`bugtype`,
  `created`). `bookings.cancelled_at` repeated `status` and was never read.

Production is rebuilt from scratch with `reset_db.sh`: the migrations were
rewritten and renumbered, so a database that ran the old ones does not
upgrade.

## Data model

```
users ─┬─ widget_settings, availability(+overrides), subscriptions, …
       ├─ event_types ── event_type_questions (question)
       ├─ contacts (email unique per host, case-insensitive)
       │     ├─ leads ── lead_answers (question, answer)
       │     │     └── calling_visits (page, UTM tags)
       │     └─ bookings ── event_types, leads
       └─ bug_reports (unid, snake_case columns)
```

- `0009_create_event_types.sql`: `event_types` and `event_type_questions`.
- `0010_create_contacts.sql`: one row per person and host, with
  `followed_up_at` (one follow-up per person). The email may wait: the form
  saves a name before it has one.
- `0011_create_leads.sql`: `leads` points to its contact and its visit;
  `lead_answers` holds one answer per asked question.
- `0012_create_bookings.sql`: `contact_unid NOT NULL`, the event type, and the
  attendance columns from Plan 33.
- `0022_start_trials_for_existing_hosts` and the `trial_started` email are
  gone: a fresh database has no hosts from before plans.

## API

- `EventType`, `EventTypeInput` and `BookingPage` carry `questions: Question[]`.
- `LeadUpdate` sends `answers: LeadAnswer[]` and `calling_visit_unid`; an
  empty answer removes the saved one. The submit call takes no body.
- `Lead` returns `answers`, a `qualification_status` read from the investment
  answer, and the visit's page and UTM tags.
- `Booking` and the webhook payload are unchanged: `invitee_name` and
  `invitee_email` are read from the contact.
- `CreateBookingRequest` drops `calling_visit_unid`; the visit reaches the
  booking through its lead.
- `AdminBugReport` uses `bug_type`, `created_at` and the other snake_case names.

## Flow

A lead save upserts the lead, then its contact. An email another contact of
the host already has moves the lead to that contact, and the emptied contact
is deleted. A booking finds or makes the contact by its email.

## Tests

- The HTTP tests switch to `questions` and `answers`; a lead naming an unknown
  visit is a 400.
- The follow-up test (`jo@` and `JO@`) now passes through the contact merge.
- `the_development_seed_applies_after_migrations_and_again` covers the seed.

## Done when

- `grep -i alter migrations/*.sql` prints nothing.
- `reset_db.sh --yes`, the AGENTS.md checks and `SQLX_OFFLINE=true cargo build`
  pass, and `frontend/src/bindings/` and `.sqlx/` are committed.
