# Plan 28: event types (Business)

Status: implemented. Gated by `max_event_types` from Plan 27: Free and Pro
book one type, Business any number. Backend tests (47 booking, 13 billing),
frontend tests (194), build, lint and knip pass. Checked in the browser: a
Business link lists three types, a 15-minute type steps every 15 minutes
through 30-minute hours, a Pro host's second type shows "Needs Business",
and an existing link with one type still opens straight away.

## Goal

A host offers several kinds of call from one link: a 15-minute check-in, a
30-minute intro, a 60-minute deep dive, each with its own title, link,
description, booking rules and questions.

## The gap this fixes

- `widget_settings` holds one host's one page: its title, description, rules
  and questions are the host's, so every call is the same kind of call.
- `widget_settings.duration_minutes` is unused: a call's length comes from
  the weekly hours (`availability.slot_minutes`).
- The pricing page sells "several event types" on Business, marked "Soon".

## Decisions

- **Length:** a type may set its own length. Its open times then step
  through the host's hours at that length; without one, they follow the
  hours' own slot lengths, as today. Every type shares one calendar: a
  booked call of any length closes the times it overlaps.
- **Where it is edited:** a new **Event types** page in the dashboard lists
  the types; each opens an editor with title, link, description, length,
  booking rules and questions. Settings keeps what belongs to the host: the
  link, timezone, pause, theme and account.
- **Public links:** `/book/you` opens the type straight away when only one
  can be booked (every existing link keeps working), and lists the types
  when there are several. Each type is at `/book/you/intro`.

## Data model

`migrations/0018_create_event_types.sql`, a new migration so a database
that already ran 0007 and 0009 keeps its data: it creates `event_types`, turns
every host's page into their first type at `/call` (same title, description,
rules and questions; length left to the hours, as before), links every
booking to it, then drops the moved columns from `widget_settings`. Checked on
a copy of the dev data from before Plan 28: an upgraded and a fresh database
end with the same columns, constraints and indexes.

Plan 42 later folded this migration into the CREATE migrations; see
`PLAN_42_SCHEMA_NORMALIZATION.md`.

```
widget_settings  host_unid PK, public_slug, timezone, is_active,
                 meeting_link, conferencing_app, accent_color
event_types      unid PK, host_unid FK, slug (unique per host), title,
                 description, duration_minutes NULL, buffer_minutes,
                 minimum_notice_hours, booking_window_days, ask_* (5),
                 is_active, created_at
bookings         + event_type_unid FK NULL (ON DELETE SET NULL)
```

`widget_settings.duration_minutes` goes; the type's optional length takes its
place. The oldest active type is the host's first.

## API

```
GET    /api/hosts/{slug}                         HostPage: host, flags, bookable types
GET    /api/hosts/{slug}/types/{type}            BookingPage of one type
GET    /api/hosts/{slug}/types/{type}/slots
POST   /api/hosts/{slug}/types/{type}/bookings
GET    /api/host/event-types                     the host's types, for editing
POST   /api/host/event-types                     403 plan_required past the plan's limit
PUT    /api/host/event-types/{unid}
DELETE /api/host/event-types/{unid}              409 for the last one
```

`/api/hosts/{slug}/slots` and `/bookings` without a type are removed; leads
stay on the host.

## Flow

```
/book/you ──▶ HostPage ──▶ one type? ──yes──▶ booking card for it
                              │
                              no──▶ list of types ──▶ /book/you/intro
```

`BookingPage` stays the one thing slots, bookings and emails read: the host's
settings joined with one type. A booking keeps its type, so the manage link
moves a call with its own type's rules and length.

## Frontend

- `PageRef = { host, type }` (`domains/widget/cadence-api.ts`) replaces the
  host slug everywhere the widget calls the API; query keys nest the type
  under the host, so a host change clears every type at once.
- `/book/$slug` (`routes/book/$slug.tsx`) opens the only type or lists them
  (`HostTypesPage`); `/book/$slug/$type` (`routes/book/$slug_.$type.tsx`)
  books one. The confirmation page carries `type` back.
- `domains/event-types/`: the API, pure helpers (free slug, new type,
  length text, plan check) and `EventTypeForm`; routes under
  `routes/dashboard/event-types/`.
- `calendar_connected` is read from `HostPage` on the dashboard.

## Plan limits

`max_event_types`: past the limit, the oldest types stay bookable, the rest
stay saved and show as "Business" in the dashboard. Creating one past the
limit answers 403.

## Tests

- Slots step at a type's length through the hours, and a call of one type
  closes overlapping times of another.
- `/api/hosts/{slug}` lists only bookable types; a Free host lists one.
- Creating a second type on Pro is refused; on Business it works.
- The last type cannot be deleted; a booking survives its type's deletion.
- Frontend: `event-types.test.ts` (free slug, new type, length text, plan
  check). The list-or-card choice and the editor are checked in the browser,
  not by a test.

## Definition of done

- [x] Hosts edit their call on the Event types page; Settings is host-only.
- [x] Business hosts book several types from one link.
- [x] Existing `/book/you` links keep working.
- [x] Pricing page drops "Soon" from event types.
- [ ] Book and move a call of a second type on the deployed site.
