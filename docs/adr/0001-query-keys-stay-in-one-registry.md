# 0001: Query keys stay in one registry

Status: accepted, 2026-10-16

## Context

The frontend's TanStack Query keys live in `frontend/src/lib/query-keys.ts`:
ten keys, used by the `*-api.ts` query factories and by the components that
update the cache after a change. An architecture review asked whether each
domain should own its keys and its cache updates instead, so the central
registry could be deleted.

## Decision

Keep the one registry. Each cache update stays beside the only change that
causes it (saving settings updates settings, saving hours updates hours), so
moving the keys into domains would spread the same lines across more files
without putting any knowledge in one place.

The one update that repeats, refreshing a booking page's open times after a
booking, move or cancel, gets a single helper next to the open-times query
(`refreshOpenTimes`), and every caller uses it.

## Consequences

- New query keys are added to `lib/query-keys.ts`.
- When a cache update starts to repeat across components, give it a helper
  next to its query rather than moving keys around.
- Revisit if the registry grows past what one screen can show, or if a
  domain's updates start to depend on another domain's keys.
