# Plan 10: admin clarity and visible settings

Status: done.

## Goal

An admin sees what needs attention the moment `/admin` opens. A host sees the
state of their booking page on `/dashboard` and reaches any setting in one
click. Moving between the two areas is a visible control, not a menu item.

## The gap this fixes

- `/admin` was five read-only lists and an overview that repeated the sidebar.
  No numbers, no health signals, no way to see who the hosts are or whether
  emails go out.
- Every setting lives under `/dashboard`. The important ones were buried:
  Google Calendar sat below the Save button, timezone and meeting link were
  filed under "Your link", and nothing said whether a booking page worked.
- The Save button scrolled out of view, Disconnect and a link change happened
  without asking, and removing hours could not be undone.
- The hours form used the browser's native time input.

## Approach

The identity from Plans 8 and 9 stays: same tokens, type and day colours, no
new dependency, no added motion. The one new idea is status as the page: both
overviews open with a plain list of states and one action per row.

Colour carries meaning only. Attention rows use the `caution` and `negative`
tones from `StatusBadge`. `--signal` stays the "now" playhead; dawn, noon and
dusk stay time of day; funnel bars use `bg-primary`.

## API

Four read-only endpoints, each in the domain that owns the data, each behind
`AdminUser`. No migration.

| Endpoint | Domain | Returns |
| --- | --- | --- |
| `GET /api/admin/hosts` | `host_settings` | `AdminHost[]`: everyone with the Host role or a booking page, with `public_slug`, `page_active`, `has_hours`, `calendar_connected`, `has_meeting_link` |
| `GET /api/admin/emails` | `emails` | `AdminEmail[]`: the 200 most recent queue rows, unsent first, with a derived `EmailState`. Never the payload, which holds the private meeting link |
| `GET /api/admin/roleaccesses` | `roleaccesses` | `AdminRoleAccess[]`: each grant with the person's name and email |
| `GET /api/admin/calling-visits/funnel` | `calling_visits` | `VisitFunnel`: visits, forms sent and calls booked in the last 7 days |

`EmailState` is derived in `emails/service.rs`, because the queue has no
terminal failure: `Sent` once `sent_at` is set, `Retrying` after any failed
attempt, `Overdue` when due more than five minutes ago and never tried (an
unknown template, or no mail transport), otherwise `Scheduled`.

## Screens

Shell, both areas:

```
┌──────────┬───────────────────────────────────────────────┐
│ logo     │ ☰  [Dashboard | Admin]  /  Page           ◐   │
│ nav      │  page content (max-w-5xl; admin max-w-7xl)    │
│ user     │                                               │
└──────────┴───────────────────────────────────────────────┘
```

The area switch shows for admins only; hosts see the area name. The theme
toggle moved from the user menu to the top bar.

Host overview:

```
Your booking page
 Link        cadence.app/book/ana            [Copy] [Open]
 Hours       Mon to Fri, 30 min calls        [Edit hours]
 Timezone    Europe/Paris                    [Change]
 Calls       ! No place to join              [Add a meeting link]
```

Settings, one page of four linked sections (`SETTINGS_SECTIONS`):

```
Booking page         link, title, description
Time                 timezone
Where calls happen   Google Calendar  ── or ──  meeting link
Questions to ask     switches
── sticky while dirty:  Unsaved changes   [Discard] [Save changes] ──
```

Hours form, with `TimePicker` in place of the native time input:

```
From            To              Call length
┌──────────┐    ┌──────────┐    ┌────────┐
│ 9:00 AM ▾│    │ 5:00 PM ▾│    │ 30 min │
└──────────┘    └────┬─────┘    └────────┘
                ┌────┴────────────────┐
                │ Type a time         │
                │ 4:30 PM   7 h 30 min│
                │ 5:00 PM ✓       8 h │
                └─────────────────────┘
Also add to   □ Tue □ Wed □ Thu □ Fri □ Sat □ Sun
```

Admin overview:

```
Needs attention                     Last 7 days
 3 emails keep failing    [View]     Visited a booking page  412 ██████
 2 hosts have no hours    [View]     Sent the form            57 █
 5 kinds of error in 24h  [View]     Booked a call            31 █
```

## What changed

- `domains/shared/AppShell.tsx`: area switch, labelled location, theme toggle,
  skip link, `wide` for admin. `UserMenu` keeps only the account and log out.
- `domains/dashboard/BookingPageStatus.tsx` replaces the one-off "no hours"
  card. Each row loads, fails and retries on its own.
- `routes/dashboard/settings.tsx`: four sections, sticky save bar, a confirm
  before a link change. `GoogleCalendarConnection` (was `GoogleCalendarCard`)
  sits beside the meeting link, asks before disconnecting, and tells "not
  connected" from "connected, but Google did not answer".
- `routes/dashboard/availability.tsx`: `TimePicker`, "Also add to", Undo on the
  removed-hours toast, times in the device's 12 or 24 hour style.
- `domains/shared/`: `Field`, `ConfirmDialog`, `TimePicker`. New shadcn
  primitive `alert-dialog`, written against the `radix-ui` package already in
  use.
- Admin: new overview, `Hosts` and `Emails` pages, a search box on the lists
  (`?q=` opens one already narrowed), names on role accesses, and a detail
  sheet on leads and bug reports for the fields the table has no room for.
- `styles/globals.css`: a global `prefers-reduced-motion` rule, since the
  shadcn primitives animate without checking.
- Every dashboard and admin route sets a page title.

## Not done here

- Admin write actions: granting roles, banning users, cancelling bookings.
- Editing an hours rule in place. There is no update endpoint; remove and add.
- A command palette, table sorting and pagination, a recent-activity feed.

## Tests

- `backend/tests/host_booking_http.rs`: a host gets 403 on all four endpoints;
  a host with no settings row is listed with `public_slug: null`; each email
  state; the emails response has no `payload`.
- `emails/service.rs`: `EmailState` from a queue row.
- Checks from `AGENTS.md`.
- Browser walk-through at 390px and 1440px, light and dark, as admin and host.

## Definition of done

- `/admin` answers "is anything wrong" without opening a list.
- A host can tell from `/dashboard` whether people can book them and join.
- No setting is more than one click from the overview that reports on it.
