# Plan 2: auth and admin

Status: done.

## Goal

A host can register and log in. Root and Admin users can open `/admin`, while
normal hosts stay in `/dashboard`. Both protected surfaces use the same
Rustify-style shadcn sidebar and user menu.

## The gap this fixes

Authentication is implemented in `backend/src/domain/auth/`. Role ownership is
in `backend/src/domain/roleaccesses/` and `frontend/src/lib/auth.ts`. Frontend
guards live in `frontend/src/routes/__root.tsx` and
`frontend/src/routes/admin/route.tsx`.

## Approach

- Login returns an HTTP-only `session` JWT cookie. API clients may use bearer.
- `AuthUser` validates token, user existence, and active status.
- `AdminUser` wraps `AuthUser` and rejects non-elevated roles with `403`.
- Admin pages are domain-owned routes under `/admin`, not a dashboard subtree.
- Both surfaces use `SidebarProvider`, `Sidebar`, `SidebarInset`,
  `SidebarTrigger`, `Avatar`, and `DropdownMenu`.

## Data model

- `migrations/0002_create_users.sql`: users, password hash, status.
- `migrations/0003_create_roleaccesses.sql`: Root, Admin, Host grants.
- `migrations/9999____seed_data.sql`: local accounts and role grants.

Seed password: `password`.

| Email | Role |
| --- | --- |
| `root@example.com` | Root |
| `admin@example.com` | Admin |
| `host@example.com` | Host |

## API

| Method | Path | Guard | Result |
| --- | --- | --- | --- |
| POST | `/api/auth/register` | public | creates Host |
| POST | `/api/auth/login` | public | sets session cookie |
| POST | `/api/auth/logout` | public | expires session cookie |
| GET | `/api/auth/me` | logged in | `SessionUser` |
| GET | `/api/roleaccesses` | logged in | caller grants |
| GET | `/api/admin/roleaccesses` | `AdminUser` | all grants |
| GET | `/api/admin/leads` | `AdminUser` | all leads |
| GET | `/api/admin/bookings` | `AdminUser` | all bookings |
| GET | `/api/admin/calling-visits` | `AdminUser` | all visits |
| GET | `/api/admin/bug-reports` | `AdminUser` | aggregated reports |

## Flow schema

```
request
  ├─ public auth route ─▶ validate ─▶ DB ─▶ cookie + JSON
  └─ protected route
       ├─ no cookie or bearer ─▶ 401
       ├─ invalid or inactive user ─▶ 401
       └─ AdminUser without Root/Admin ─▶ 403
```

Frontend:

```
/dashboard ─▶ DashboardSidenavLayout ─▶ host pages
/admin     ─▶ hasAdminAccess ────────▶ AdminSidenavLayout ─▶ admin pages
             false ─────────────────▶ redirect /dashboard
```

## Impl sketch

- `auth/handlers.rs`: register, login, logout, current user.
- `auth/middleware.rs`: `AuthUser`, `AdminUser`, cookie and bearer extraction.
- `roleaccesses/db.rs`: role queries only.
- `frontend/src/lib/auth.ts`: role constants and session bootstrap.
- `frontend/src/domains/dashboard/components/dashboard-sidenav.tsx`: host UI.
- `frontend/src/domains/admin/components/admin-sidenav.tsx`: admin UI.
- `frontend/src/domains/dashboard/components/user-dropdown-sidenav.tsx`: shared menu.

## Out of scope

- Password reset and email verification, phase 5.
- OAuth providers, phase 4 or 6.
- Fine-grained resource permissions, phase 7.

## Tests

- Register valid host.
- Reject invalid email, short password, and duplicate email.
- Login valid and invalid credentials.
- `/api/auth/me` with no cookie returns `401`.
- Admin endpoint with Host returns `403`.
- Admin endpoint with Root/Admin returns `200`.
- Logout expires the cookie.
- Route build includes `/admin`, never `/dashboard/admin`.

## Milestones

- A0: users and role grants, done.
- A1: JWT cookie and bearer auth, done.
- A2: AdminUser authorization matrix, done.
- A3: `/dashboard` and `/admin` sidebars, done.
- A4: generated bindings and quality gates, done.

## Definition of done

- Host cannot read admin data.
- Admin and Root can read admin data.
- Logged-out requests do not reach protected handlers.
- Both surfaces use the same sidebar interaction model.
- Clippy, build, typecheck, Biome, and Knip pass.

## After phase 2

Phase 3 completes the booking funnel and fills local dashboards with useful data.
