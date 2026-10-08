# Plan 7: per-host booking links

Status: done.

## Goal

A host registers, sets the hours they take calls, and gets their own link,
`/book/{slug}`. Anyone with the link books a call in those hours. The host
chooses the page title and description, their timezone, and which optional
questions the form asks. Hosts never see or change one another's data.

## The gap this fixes

- Every public endpoint resolved "the" host through
  `availability::service::configured_public_host`, the most recently updated
  active `widget_settings` row. Only the seed created such a row, so a host
  who registered had no link, and with two rows requests would go to
  whichever host saved last.
- The times shown to visitors ignored `/dashboard/availability`. They were a
  fixed list (13:00 to 18:30 Paris) minus Google busy times, and failed when
  Google was not connected.
- The form always asked five lead-qualification questions, hardcoded in
  `frontend/src/domains/widget/booking-form.ts`.
- `POST /api/google-calendar/login` and `GET /api/google-calendar/events`
  were public and acted on the configured host, not the caller.
- `leads::db::upsert` updated a lead by id without checking its host.

## Approach

- `widget_settings` already had one row per host with `public_slug`, `title`,
  `description` and `timezone`. A new `host_settings` domain reads and writes
  it. The row is created on the host's first `GET /api/host/settings`, which
  covers new, existing and seeded users with one code path and no backfill.
- Public routes carry the slug: `/api/hosts/{slug}/...`. Each handler
  resolves it with `host_settings::service::public_page`, so unknown,
  switched-off and banned hosts all return the same 404. The un-scoped routes
  and `configured_public_host` are deleted.
- Slots come from the host's availability rules, read as wall-clock times in
  the host's timezone. Google Calendar is an optional overlay: when connected
  it removes busy times and adds a Meet link. A broken connection is logged
  and the page keeps working from the host's hours.
- Unused columns stay unused: `accent_color`, `duration_minutes`,
  `buffer_minutes`, `minimum_notice_hours`. Call length is the rule's
  `slot_minutes`.

## Data model

`migrations/0017_add_widget_question_flags.sql`:

```sql
ALTER TABLE widget_settings
    ADD COLUMN ask_currently_working_on BOOLEAN NOT NULL DEFAULT false,
    ADD COLUMN ask_how_heard_about_us   BOOLEAN NOT NULL DEFAULT false,
    ADD COLUMN ask_what_stopping_you    BOOLEAN NOT NULL DEFAULT false,
    ADD COLUMN ask_investment_comfort   BOOLEAN NOT NULL DEFAULT false,
    ADD COLUMN ask_urgency_level        BOOLEAN NOT NULL DEFAULT false,
    ADD CONSTRAINT widget_public_slug_format CHECK (
        public_slug ~ '^[a-z0-9]+(-[a-z0-9]+)*$'
        AND char_length(public_slug) BETWEEN 3 AND 40
    );
```

- Flags are named after the `leads` columns they gate. Existing rows get
  `false`: a page asks for name, email and phone until its host says more.
- The migration does not depend on the seed, so it runs before it on a fresh
  database and after it on an existing one.
- New rows are created with `timezone = 'UTC'`, because hours saved before
  this plan were entered as UTC. The host picks their real timezone in
  settings.
- Slug: slugified full name, else the email's local part, else `host`; on a
  collision a `-xxxx` suffix. The host can change it; old links then 404.

`migrations/0018_add_meeting_link.sql` adds `widget_settings.meeting_link`
(nullable): the host's own place to meet, such as a Zoom or Meet room. A
booking's link is Google's Meet link when the calendar is connected and the
event was created, otherwise this one. It is stored in
`bookings.google_meet_url`, which keeps its name because the seed inserts
into it, and is exposed on the wire as `Booking.meeting_url`.

## API

| Method | Path | Guard | Request | Response | Errors |
| --- | --- | --- | --- | --- | --- |
| GET | `/api/hosts/{slug}` | public | | `BookingPage` | 404 |
| GET | `/api/hosts/{slug}/slots` | public | | `TimeSlot[]` | 404 |
| POST | `/api/hosts/{slug}/bookings` | public | `CreateBookingRequest` | `Booking` | 400, 404, 409 |
| POST | `/api/hosts/{slug}/leads/{lead}` | public | `LeadUpdate` | `Lead` | 400, 404 |
| POST | `/api/hosts/{slug}/leads/{lead}/submit` | public | `SubmitLeadRequest` | `Lead` | 400, 404 |
| GET | `/api/host/settings` | logged in | | `HostSettings` | 401 |
| PUT | `/api/host/settings` | logged in | `HostSettings` | `HostSettings` | 400, 401, 409 |
| GET | `/api/google-calendar/oauth-config` | logged in | | client id, redirect URI | 401 |
| POST | `/api/google-calendar/login` | logged in | code | ok | 400, 401 |
| GET | `/api/google-calendar/events` | logged in | | `CalendarEvent[]` | 400, 401 |

`BookingPage` carries the host's name, page text, timezone, the five `ask_*`
flags, `calendar_connected` and `has_meeting_link`. It never carries an id,
an email or the meeting link itself; the link goes only to people who booked.
`HostSettings.meeting_link` must be an https URL of at most 500 characters.

Removed: `/api/leads/*`, `/api/hosts/{host_unid}/*`,
`/api/google-calendar/available-slots`, `available-slots-batch` and `book`.

## Flow schema

```
register ─▶ /dashboard/availability (hours) ─▶ /dashboard/settings (link,
            timezone, questions, optional Google) ─▶ share /book/{slug}

visitor ─▶ /book/{slug}
   │  GET /api/hosts/{slug}            page text, question flags
   │  GET /api/hosts/{slug}/slots      rules in host timezone
   │                                   - inside 4h notice
   │                                   - already booked
   │                                   - Google busy (if connected)
   ├─ pick a time ─▶ details form (asked questions only)
   │  POST .../leads/{lead}            answers to unasked questions dropped
   │  POST .../bookings                slot re-checked, Meet event if
   │                                   connected, emails queued
   └─▶ /booking-confirmation
```

```
/dashboard/settings
┌──────────┬────────────────────────────────────────────┐
│ Overview │ Settings                                   │
│ Bookings │ ┌ Your link ─────────────────────────────┐ │
│ Availab. │ │ host/book/[ giorgi           ] Copy    │ │
│ Settings │ │ Title, Description, Timezone           │ │
│          │ └────────────────────────────────────────┘ │
│          │ ┌ Questions to ask ──────── five switches┐ │
│          │ └────────────────────────────────────────┘ │
│          │ [Save changes]                             │
│          │ ┌ Google Calendar (optional) ────────────┐ │
│          │ └────────────────────────────────────────┘ │
└──────────┴────────────────────────────────────────────┘
```

## Implementation sketch

- `backend/src/domain/host_settings/`: `HostSettings`, `BookingPage`,
  `get_or_create`, `update`, `public_page`, slug rules.
- `availability/service.rs`: `slots_between(rules, timezone, from, to)` and
  `local_to_utc`. Pure, unit tested.
- `bookings/service.rs`: `public_slots` (rules, notice, booked, busy) and
  `create`, which re-checks the slot, checks the lead belongs to the host,
  creates the Meet event when connected, and queues the emails.
- `google_calendar/service.rs`: `busy_intervals` and `create_meet_event`.
  The fixed slot list and the old slot and booking code are gone.
- `leads/service.rs`: `keep_asked_answers`; `leads/db.rs`: upsert only
  updates a lead of the same host.
- Frontend: `routes/book/$slug.tsx`, `routes/dashboard/settings.tsx`,
  `domains/widget/booking-form.ts` (`QUESTIONS`, shared by the form and the
  settings page), `domains/widget/zoned-time.ts`, one slots query per page
  grouped by the visitor's local day. `/book-a-call` is removed; `/` is the
  landing page (Plan 9). The Google card moved from `/admin` to
  `/dashboard/settings`, so `GOOGLE_REDIRECT_URI` is now
  `/dashboard/calendar/callback`.
- New dev-dependency `tower` (`util`): the first HTTP tests drive the router
  in-process. New shadcn primitive: `switch`.

## Out of scope

- **OAuth `state`** through the unused `oauth_states` table. Plan 5.
- **`host_unid` on `calling_visits`** for a per-host funnel. Later plan.
- **Showing answers** to the host or in emails, and per-host question
  wording or options. Needs a detail view first.
- **Redirects from an old slug** after a rename, and a switch for
  `is_active`.
- **Buffer and notice settings.** Notice is a 4 hour constant.
- **Teams, round robin, delegated admins, custom domains, paid plans.**

## Tests

`backend/tests/host_booking_http.rs`:

- The public page has no id or email and asks nothing by default; links are
  case-insensitive; unknown and switched-off links are 404.
- Settings need a session; the first read creates a link and later reads
  keep it; two hosts with one name get different links.
- A bad slug, empty title or unknown timezone is 400; a taken slug is 409; a
  save does not touch another host's settings; the old link stops working.
- Answers to questions that are off are stored as NULL, with no
  qualification status; turning a question on keeps its answer.
- A lead cannot be saved or submitted through another host's link.
- Slots follow the host's hours and timezone with no Google connection; a
  host with no hours gets an empty list.
- Only offered times can be booked; the lead must be the host's; a booked
  time is gone and booking it again is 409.
- Calendar login, events and config are 401 logged out; the old public
  routes are 404.
- A meeting link that is not https, not a URL or too long is 400; an empty
  one is stored as null; the public page says a link exists without
  showing it; a booking without Google carries the host's link and so does
  every queued email.

Unit tests: slug rules in `host_settings/service.rs`, timezone and window
cases in `availability/service.rs`.

## Milestones

- H0: migration, `host_settings` domain, bindings.
- H1: host-scoped public API, isolation and calendar auth fixes, `/book/$slug`.
- H2: slots and bookings from the host's hours, Google optional.
- H3: question switches in the form and `/dashboard/settings`.
- H4: HTTP tests and docs.

## Definition of done

- A new host with hours and no Google connection can be booked from their
  own link.
- Host A cannot read or change host B's settings, leads or bookings.
- With every switch off the form asks only for name, email and phone.
- Checks from `AGENTS.md`, `cargo test`, clean `frontend/src/bindings/` and
  `.sqlx/`.

## After phase 7

Plan 5 (test hardening) picks up OAuth `state` and the wider auth matrix.
