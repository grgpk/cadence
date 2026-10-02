# Plan 4: calendar and email delivery

Status: implemented.

## Rustify-compatible provider behavior

Cadence public booking must preserve the Rustify provider contract: timezone-aware
availability, busy-event filtering, a server-side booking re-check, Google Meet
creation, and confirmation/reminder delivery through `email_queue`.

## Goal

A host connects Google Calendar once. Visitors never receive busy slots. A
confirmed booking creates a Google event with Meet, then confirmation and
reminder emails are delivered through Resend.

## The gap this fixes

`migrations/0011_create_google_calendar_config.sql` and
`migrations/0010_create_email_queue.sql` exist. Google Calendar OAuth, token
refresh, busy-event filtering, Meet event creation, and queued confirmation
reminders are now implemented under `backend/src/domain/google_calendar/`.

## Approach

- Keep OAuth tokens server side in `google_calendar_config`.
- Fetch busy intervals, generate candidate slots, then filter busy intervals.
- Re-check the selected slot before creating the event.
- Keep all mail asynchronous through `email_queue`.
- Use `RESEND_API_KEY`, `RESEND_SMTP_HOST`, `RESEND_SMTP_PORT`,
  `RESEND_SMTP_USERNAME`, and `EMAIL_FROM`.

## Data model

Use existing `google_calendar_config`, `oauth_states`, booking event columns,
and `email_queue`. Add provider error metadata only in a new migration if
`attempts` and `last_error` prove insufficient.

## API

| Method | Path | Guard | Result |
| --- | --- | --- | --- |
| GET | `/api/google-calendar/connect` | `AdminUser` | OAuth redirect |
| GET | `/api/google-calendar/callback` | public state | connected result |
| POST | `/api/google-calendar/disconnect` | `AdminUser` | disconnected |
| GET | `/api/google-calendar/available-slots` | public | calendar-filtered slots |
| POST | `/api/google-calendar/book` | public | booking plus Meet URL |
| GET | `/api/admin/email-queue` | `AdminUser` | queue status |

## Flow schema

```
admin ─▶ OAuth state ─▶ callback ─▶ calendar config
visitor ─▶ rules ─▶ busy intervals ─▶ free slots
visitor ─▶ booking ─▶ re-check ─▶ event + Meet ─▶ queue
worker ─▶ due rows ─▶ Resend SMTP ─▶ sent or failed
```

## Out of scope

- Multiple calendars per host, phase 7.
- Outlook, CalDAV, Zoom, and template editor, later.

## Tests

- OAuth state expiry and one-time consumption.
- Busy intervals, DST, and timezone conversion.
- Booking conflict after calendar re-check.
- Queue send, retry, and terminal failure.
- Provider errors never leak SMTP details to clients.

## Milestones

- C0: calendar domain and OAuth service. Done.
- C1: busy-time filtering. Done.
- C2: event and Meet persistence. Done.
- C3: Resend worker and queued reminders. Done.
- C4: provider integration tests and quality gates. Partial, live provider smoke test requires configured Google credentials.

## Definition of done

- Busy slots never appear.
- Double booking fails safely.
- Successful booking has Meet URL and queue rows.
- Failed delivery is visible and retryable.

## After phase 4

Phase 5 hardens every public and protected contract.
