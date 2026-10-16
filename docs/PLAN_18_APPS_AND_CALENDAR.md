# Plan 18: apps and bookings calendar

Status: implemented. Frontend only: no endpoint, migration or wire type changed.

## Goal

Two screens look like the references in `docs/app-system/` and
`docs/booking-calendar-view/`:

- **Apps.** A collapsible **Apps** group in the sidebar holds an **App store**
  (search, categories, app cards with Details), a page for each app (preview,
  status, description, pricing, set up) and **Installed apps** (the old
  Integrations page).
- **Bookings calendar.** The bookings "Week" strip becomes **Calendar**, a
  Google Calendar style grid with **Day**, **Week** and **Month**.

## The gap this fixes

- Integrations was one page with coming-soon chips. There was no place to
  browse apps or read about one, and no room for more.
- The week view laid days out as horizontal strips. Hosts read calendars as
  columns of hours, and there was no day or month view.

## Layout

```
┌──────────────┬──────────────────────────────────────────────┐
│ Overview     │ App store                        [🔍 Search] │
│ Bookings     │ Categories  [📅 Calendar 3] [🎥 Conferencing 4]│
│ Availability │ Available   [card] [card] [card]             │
│ Apps ▾       │ Coming soon [card] [card] [card] [card]      │
│   App store  │                                              │
│   Installed  │ /apps/$slug: ← App store                     │
│ Settings     │ [preview panel]  [logo] Name · badges        │
│              │                  description, pricing, setup │
└──────────────┴──────────────────────────────────────────────┘

Bookings ▸ Calendar
┌ 12 to 18 October [Today] ‹ ›                 [Day|Week|Month] ┐
│      MON 12  TUE 13  WED 14  THU 15  (FRI 16)  SAT 17  SUN 18 │
│ 9 AM ░open░  ░░░░░░  ░░░░░░  ░░░░░░  ●───────                 │
│10 AM ░░░░░░  ▌Anna   ░░░░░░  ░░░░░░  ░░░░░░                   │
└───────────────────────────────────────────────────────────────┘
Month: six weeks, Monday first, up to 3 calls a day then "+N more".
```

## Data model and API

No change. The app catalog is a frontend constant (`domains/integrations/catalog.ts`).
Install state is derived: Google Calendar from `calendar_connected`, Google Meet
and custom link from `HostSettings.conferencing_app` and `meeting_link`.

## Flow

- `/dashboard/apps?category=` filters the store; search filters by name and summary.
- `/dashboard/apps/$slug` sets up Google Calendar (`GoogleCalendarConnection`)
  or a meeting app (`CallFlow` + `ConferencingForm`). An unknown slug is a 404.
- `/dashboard/apps/installed` replaces `/dashboard/integrations`; the OAuth
  callback and booking page status link there.
- `/dashboard/bookings?view=calendar&mode=week&offset=-1` keeps the calendar's
  place in the URL. Without a mode, a phone opens Day and a wider screen Week.
  Clicking a month day opens that day.

## Implementation sketch

- `components/ui/collapsible.tsx` (shadcn) for the Apps group. `NavItem` takes
  optional `items`; the shell renders them with `SidebarMenuSub`. Collapsed to
  icons, the group links to its first page.
- `domains/integrations/app-card.tsx`: `AppLogo`, `AppStatus`, `AppCard`, `CategoryTile`.
- `domains/bookings/calendar-view.tsx` replaces `week-view.tsx`: `TimeGrid`
  for day and week, `MonthGrid` for month. Open hours reuse `phaseSpans` and
  `OPEN_STYLES`; positions reuse `placeOnWeek`.
- `bookings.ts` gains `dayOf`, `daysBetween`, `monthGrid` and `lanes` (side by
  side columns for overlapping calls), all on UTC-midnight calendar days.

## Tests

- `catalog.test.ts`: install state, search and category filter.
- `bookings.test.ts`: `dayOf`, `daysBetween` across a clock change, `monthGrid`
  (month starting on a Sunday, year end, Paris clock change), `lanes`.

## Milestones

1. Catalog, store, app page, installed apps, Apps nav group.
2. Calendar view with day, week and month.

## Definition of done

- Every check in AGENTS.md passes.
- Sidebar Apps group opens on its pages; breadcrumbs name the sub page.
- Calendar works at phone width (week scrolls sideways) and in both themes.
