# Plan 17: integrations

Status: implemented. The Google Calendar and Google Meet paths were not run
against the real Google services.

## Goal

A host has one place, **Integrations**, to connect a calendar and choose where
calls happen, like cal.com's apps page. Google Calendar lives there. Google
Meet becomes a meeting app the host picks, next to their own link. Outlook,
Apple Calendar, Zoom and Microsoft Teams are shown as coming soon.

## The gap this fixes

- Google Calendar sat inside Settings, under "Where calls happen", next to a
  free meeting link field.
- Which link a booking got was implicit: Meet whenever a calendar was
  connected, otherwise the link. A host with Google Calendar could not use
  their own Zoom room and still have busy times blocked.
- There was no place for more providers.

## Layout

```
┌──────────────┬──────────────────────────────────────────────┐
│ Overview     │ Integrations                                 │
│ Bookings     │                                              │
│ Availability │ Calendars                                    │
│ Integrations │ ┌───────────────────┐ ┌───────────────────┐  │
│ Settings     │ │ Google Calendar   │ │ Outlook Calendar  │  │
│              │ │ ● Connected · 4   │ │ Coming soon       │  │
│              │ └───────────────────┘ └───────────────────┘  │
│              │ ┌───────────────────┐                        │
│              │ │ Apple Calendar    │                        │
│              │ └───────────────────┘                        │
│              │ Conferencing                                 │
│              │ ┌──────────────────────────────────────────┐ │
│              │ │ Default meeting app                      │ │
│              │ │ ◉ Google Meet      ○ Your own link       │ │
│              │ │ Meeting link [https://…]         [Save]  │ │
│              │ └──────────────────────────────────────────┘ │
│              │ ┌───────────────────┐ ┌───────────────────┐  │
│              │ │ Zoom  Coming soon │ │ Teams Coming soon │  │
│              │ └───────────────────┘ └───────────────────┘  │
└──────────────┴──────────────────────────────────────────────┘
```

## Data model

`widget_settings.conferencing_app TEXT NOT NULL DEFAULT 'google_meet'`, checked
to `google_meet | custom_link`. Rust `ConferencingApp` gives the closed set;
the column stays a string for sqlx, like `BookingStatus`.

The column is folded into `migrations/0007_create_widget_settings.sql` rather
than added with an `ALTER` migration. That breaks the "never edit a migration"
rule on purpose, the same way Plan 16 folded every `ALTER`. **Every database,
production included, must be rebuilt** (`reset_db.sh --yes` locally), or sqlx
refuses to start on the changed checksum of 0007.

## API

| Method | Path | Body | Notes |
|---|---|---|---|
| `PUT` | `/api/host/conferencing` | `ConferencingUpdate { app, meeting_link }` | `custom_link` needs an https link (400). Returns `HostSettings`. |

- `HostSettings` and `BookingPage` carry `conferencing_app`.
- `PUT /api/host/settings` no longer writes `meeting_link` or
  `conferencing_app`, so the Settings form and the Integrations page never
  overwrite each other.
- Google Meet may be picked before a calendar is connected. New hosts start
  on it.

## Flow

Booking create (`bookings/service.rs`) picks where a call happens. The
frontend's `callPlace` (`domains/integrations/conferencing.ts`) uses the same
rule:

- `google_meet` with a calendar: a calendar event with a Meet. If Google makes
  no Meet, the call uses the host's link.
- `custom_link` with a calendar: a calendar event without `conferenceData`
  (busy time and invite), and the call uses the host's link.
- No calendar: the host's link, if there is one.

`GoogleCalendarClient::create_event` takes `with_meet` instead of always
asking for a Meet.

Disconnecting Google Calendar does not change `conferencing_app`. While Meet
is picked without a calendar, calls fall back to the host's link. The
Integrations page and the overview "Calls" row say so.

## Frontend

- `ROUTES.dashboardIntegrations`, a sidebar entry (`Blocks` icon), and
  `routes/dashboard/integrations.tsx`.
- `domains/integrations/` holds:
  - `catalog.ts`: the coming-soon apps
  - `integration-card.tsx`
  - `conferencing-form.tsx`
  - `conferencing.ts`: `CONFERENCING_APP` and `callPlace`
  - `google-meet-icon.tsx`
  - `google-calendar-connection.tsx`, moved from `domains/dashboard/`
- The Google sign-in callback (`/dashboard/calendar/callback`, registered with
  Google, so unchanged) returns to Integrations.
- Settings loses the "Where calls happen" section and its anchor. The overview
  "Calls" row links to Integrations.
- The booking widget labels the call through `callPlace`.

## Left alone

- `bookings.google_event_id` and `google_meet_url` keep their names;
  `meeting_url` is already the wire name.
- Tokens are still stored as plain text (Plan 16 known gap).
- There is one calendar per host, and there are no event types.

## Follow-ups

- Plan 18, Outlook Calendar and Teams via Microsoft Graph: busy times, events,
  and Teams meetings. This plan brings in a provider-neutral credentials table
  (`calendar_connections` keyed by host and provider) to replace
  `google_calendar_config`.
- Plan 19, Zoom: OAuth app and a meeting per call, adding `zoom` to
  `conferencing_app`.
- Apple / CalDAV calendars.

## Tests

- Unit tests, `host_settings/service.rs`:
  - the link is trimmed, and an empty link counts as none
  - the `ConferencingApp` wire and column values
- HTTP tests, `host_booking_http.rs`:
  - conferencing validation, including that a custom link needs the link
  - the Settings PUT cannot overwrite the link or the app
  - the public page shows `conferencing_app`
  - a booking without Google uses the host's link
- Vitest, `conferencing.test.ts`: `callPlace`.
- `request.http`: `PUT /api/host/conferencing`.

## Definition of done

- [x] The Integrations page is in the sidebar, with Google Calendar and the
  coming-soon cards.
- [x] The default meeting app is saved per host and used at booking.
- [x] Settings no longer duplicates the call settings.
- [x] Bindings regenerated and the `.sqlx` cache updated.
- [x] Backend tests, frontend build, typecheck, lint, knip and test pass.
- [ ] Manual run against real Google: Meet on, Meet off with a calendar.
- [ ] Production database rebuilt for the edited 0007.
