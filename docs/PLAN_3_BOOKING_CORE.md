# Plan 3: booking core

Status: in progress.

## Cadence widget parity

The public widget follows the proven Rustify two-step flow while remaining owned
by Cadence: lead qualification first, calendar booking second. The visual
language, field progression, timezone choice, calendar interaction, loading
states, errors, confirmation redirect, and attribution fields are all part of
this phase. Brand copy must say Cadence everywhere.

## Goal

A visitor sees recurring host slots, submits a lead, books a slot, and the host
sees the booking in `/dashboard`. Admin sees the funnel across leads, bookings,
and calling visits.

## The gap this fixes

The baseline domains exist in `backend/src/domain/leads/`, `availability/`,
`calling_visits/`, and `bookings/`. The public widget is
`frontend/src/domains/widget/BookingWidget.tsx`. Remaining work is richer
development data, complete booking UX, and end-to-end contract tests.

## Approach

- Availability rules generate slots server side.
- The widget uses the existing public API and TanStack-compatible request flow.
  Local state owns form fields, step state, calendar selection, and errors.
- A booking links optional lead and visit IDs and queues mail.
- Admin reads remain owned by the relevant domain.

## Data model

Existing tables: `calling_visits`, `leads`, `widget_settings`, `availability`,
`bookings`, and `email_queue`. Rich seed data must use fixed UUIDs,
`ON CONFLICT DO NOTHING`, and future relative booking timestamps. Never edit an
applied migration in a shared environment. Add a new seed migration instead.

## API

| Method | Path | Guard | Result |
| --- | --- | --- | --- |
| POST | `/api/calling-visits` | public | visit identifier |
| POST | `/api/leads/{lead}` | public | lead |
| POST | `/api/leads/{lead}/submit` | public | submitted lead |
| GET | `/api/google-calendar/available-slots` | public | available slots |
| POST | `/api/google-calendar/book` | public | created booking |
| GET | `/api/availability` | logged in | host rules |
| GET | `/api/bookings` | logged in | host bookings |
| GET | `/api/admin/leads` | `AdminUser` | all leads |
| GET | `/api/admin/bookings` | `AdminUser` | all bookings |
| GET | `/api/admin/calling-visits` | `AdminUser` | all visits |

## Flow schema

```
widget
  ├─ GET slots ─▶ availability service ─▶ free UTC slots
  ├─ POST visit ─▶ calling_visits
  ├─ POST lead ──▶ leads
  └─ POST booking
       ├─ validate slot against rules
       ├─ insert booking with host/slot uniqueness
       └─ enqueue confirmation and future reminders
```

## Impl sketch

- `availability/service.rs`: slot generation and notice window.
- `leads/db.rs`: upsert and submit queries.
- `bookings/service.rs`: validation, transaction, queue writes.
- `frontend/src/lib/query-keys.ts`: stable query keys.
- `BookingWidget.tsx`: shadcn form, loading, error, and success states.
- `CadenceBookingWidget.tsx`: two-step Cadence-branded qualification and booking
  widget, matching the Rustify reference behavior.

## Out of scope

- Google busy-time filtering and Meet links, phase 4.
- Rich admin tables and filters, phase 5.
- Multi-host URL onboarding, phase 7.
- External embed loader, phase 6.

## Tests

- Weekday, notice-window, and occupied-slot generation.
- Lead upsert then submit.
- Booking happy path creates queue row.
- Duplicate host/slot returns conflict.
- Invalid email and invalid slot rejected.
- Host list never returns another host's bookings.
- Admin list returns all rows only to `AdminUser`.

## Milestones

- B0: schema and generated models, done.
- B1: availability, leads, visits, bookings API, done.
- B2: public widget and host dashboard query migration, done.
- B3: rich deterministic development seed, done.
- B4: contract tests and complete error states, next.

## Definition of done

- Local seed shows useful data in every dashboard page.
- Booking transaction writes booking plus queue atomically.
- Duplicate slots are rejected.
- Host isolation and admin authorization are tested.

## After phase 3

Phase 4 adds Google Calendar and production-ready Resend delivery.
