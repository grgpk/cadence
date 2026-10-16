# Plan 28: event types (Business)

Status: planned. Gated by `max_event_types` from Plan 27.

## Approach

- Split `widget_settings`:
  - host-level fields stay: `public_slug`, timezone, accent, `is_active`, conferencing
  - a new `event_types` table takes the per-type fields: id, `host_unid`, slug, title, description, duration, buffer, notice, window, `ask_*`, position, `is_active`
  - `bookings` gets `event_type_unid`
- Following the user's preference, the schema change goes into the existing CREATE migrations (`0007`, `0009`) and the databases are reset, instead of adding ALTER migrations.
- Public URLs:
  - `/book/$slug` lists the types, or opens the only one.
  - `/book/$slug/$type` books that type.
  - The slot policy takes the type's settings.
- The limit is `max_event_types`. On a downgrade, only the first type (by position) stays bookable; the others stay saved but show as "Business".
- Dashboard: a new "Event types" nav item with a list, create and edit.

