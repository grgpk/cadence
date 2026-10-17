# Plan 27: pricing, plans and billing

Status: done. Plans and the 14-day Pro trial, the public `/pricing` page (a paid pick is carried through sign-up to `/dashboard/billing`), Stripe Checkout, the Customer Portal and signed webhooks, the Free plan's limits with their emails, and the admin Plans page. Run against Stripe test mode on 2026-10-17, with `stripe listen` forwarding to the backend: checkout made a real session, a subscription with `user_unid` metadata turned a trialing host into Pro/month, a switch to Business/year unlocked event types and webhooks, cancel at period end showed, a replayed event was acknowledged and ignored, a second checkout got 409, and the deleted subscription left the host on Free. Not run live: a failed renewal (`past_due`), which needs a test clock; it is covered by the fake-Stripe tests. Business features follow in Plan 28 (event types) and Plan 29 (webhooks).

## Goal

Cadence has no way to make money: every host gets everything, forever. We add a
Calendly-style public `/pricing` page, real Stripe subscriptions, a 14-day Pro
trial without a card, and entitlements that the backend enforces. Business
gets two new features that do not exist yet: multiple event types and
webhooks.

Decisions already made (from Q&A):
- Full billing with **Stripe** (Checkout + Customer Portal + webhooks).
- Tiers **Free / Pro / Business**, billed **per user** (no teams/seats), **monthly or yearly**.
- Prices: **Pro $10/mo or $8/mo billed yearly ($96)**, **Business $16/mo or $13/mo yearly ($156)**.
- **14-day Pro trial, no card**, for every new signup; drops to Free unless they subscribe.
- On downgrade or a failed payment: **keep the data, stop applying it**. Re-upgrading restores it.

What the exploration found:
- There is no billing, plan, team or Stripe code anywhere. One user is one host is one booking page (`widget_settings`, keyed by `host_unid`).
- Reminders (24h/2h/30m) are queued unconditionally in `backend/src/domain/emails/service.rs` (`queue_for_invitee`, ~line 234).
- `ROUTES` (`frontend/src/lib/routes.ts`), `QUERY_KEYS` (`frontend/src/lib/query-keys.ts`), `PublicLayout`, `AppShell` + `DASHBOARD_NAV`, `SegmentedControl`, `PanelCard`, `PageHeader`, and the shadcn `table`/`badge`/`card` components can all be reused. There is no Accordion yet.
- The next plan number is 27. Event types and webhooks are each big enough to get their own phase file (28 and 29), in line with "one PLAN file per phase".

## Tier matrix (single source of truth: `backend/src/domain/billing/plans.rs`)

| Entitlement | Free | Pro (trial) | Business |
|---|---|---|---|
| Bookings per month (host timezone) | 25 | unlimited | unlimited |
| Invitee reminders | 24h | 24h, 2h, 30m | 24h, 2h, 30m |
| Custom booking questions (`ask_*`) | – | yes | yes |
| Custom accent colour (soon) | – | yes | yes |
| "Booked with Cadence" badge | shown | hidden | hidden |
| Google Calendar, Meet, own link, date overrides, reschedule/cancel links | yes | yes | yes |
| Event types | 1 | 1 | unlimited (Plan 28, done) |
| Webhooks | – | – | yes (Plan 29, done) |

## Data model: `migrations/0017_create_subscriptions.sql` (new tables only)

```sql
CREATE TABLE subscriptions (
  user_unid              UUID PRIMARY KEY REFERENCES users(unid) ON DELETE CASCADE,
  plan                   TEXT NOT NULL,          -- 'pro' | 'business'
  status                 TEXT NOT NULL,          -- trialing | active | past_due | canceled | incomplete | comped
  billing_interval       TEXT,                   -- 'month' | 'year', NULL for trial/comped
  trial_ends_at          TIMESTAMPTZ,
  current_period_end     TIMESTAMPTZ,
  cancel_at_period_end   BOOLEAN NOT NULL DEFAULT false,
  stripe_customer_id     TEXT UNIQUE,
  stripe_subscription_id TEXT UNIQUE,
  stripe_synced_at       TIMESTAMPTZ,            -- event.created of last applied event (stale-event guard)
  created_at / updated_at TIMESTAMPTZ ...,
  CHECK constraints on plan, status, billing_interval
);
CREATE TABLE stripe_events (id TEXT PRIMARY KEY, type TEXT NOT NULL, received_at TIMESTAMPTZ NOT NULL DEFAULT now());
```

A user with no row is on **Free**. Registration inserts `plan='pro', status='trialing', trial_ends_at = now() + 14 days`. The dev seed gets one host on each tier.

**Effective plan** is a pure function, `effective_plan(sub, now)`:
- `trialing` and `trial_ends_at > now` gives Pro.
- `active`, `past_due` (the grace period while Stripe retries) or `comped` gives `plan`.
- Anything else gives Free.

## Backend: new domain `backend/src/domain/billing/`

The domain follows the usual `routes -> handlers -> service -> db` layering.

- `plans.rs` holds three types, all deriving `TS` and exported to `frontend/src/bindings/`:
  - `enum Plan { Free, Pro, Business }` and `enum BillingInterval { Month, Year }`
  - `struct Entitlements { max_bookings_per_month: Option<u32>, reminders: Vec<Reminder>, custom_questions, custom_accent, hide_branding, max_event_types: Option<u32>, webhooks }`

  `Plan::entitlements()` is a `const` table. A `PlanCatalog` holds the display prices in cents and the Stripe price ids, read from env.
- `stripe.rs` is a thin `reqwest` client with two form-encoded calls: create a Checkout Session and create a Portal Session. It also verifies `Stripe-Signature` with HMAC-SHA256 and a 5-minute tolerance.
  - **New deps: `hmac`, `sha2`, `hex`.** We avoid `async-stripe` because it is a very large compile for only two calls and a webhook.
  - `Payments` (`stripe.rs`) is the seam: Stripe in production, a fake with made-up URLs in tests. `AppState.payments` is `None` when a Stripe key or price is missing; checkout and the portal then answer 503.
- `service.rs` does four jobs:
  - `entitlements_for(user)` returns the entitlements for a user's effective plan.
  - `start_trial(tx, user)` is called from `auth::service` registration.
  - `checkout(user, plan, interval)` passes the known Stripe customer, or else the host's email so Checkout makes the customer on payment (no customer API call), with `subscription_data.metadata.user_unid`, in `mode=subscription`. A host Stripe already bills gets 409 and goes to the portal. The session returns to `success_url={APP_URL}/dashboard/billing?checkout=done` and `cancel_url={APP_URL}/pricing`.
  - `apply_event(event)` dedupes on `stripe_events` and ignores stale events by comparing `event.created` with `stripe_synced_at`. It handles:
    - `customer.subscription.created`, `.updated` and `.deleted`; they carry everything, including a failed renewal as `past_due`, so `checkout.session.completed` is not needed
    - every other event is recorded and acknowledged
    - The end of a subscription the host has since replaced changes nothing.
    - The payment-failed host email comes with milestone 4.
  - It maps the Stripe price id to `(plan, interval)`.
- `routes.rs` exposes these endpoints:
  - `GET  /api/billing/plans` (public): the catalog with prices and entitlements. The pricing page is server-rendered from it.
  - `GET  /api/billing/me` (`AuthUser`): `BillingSummary { plan, status, interval, trial_ends_at, current_period_end, cancel_at_period_end, entitlements, usage: { bookings_this_month } }`
  - `POST /api/billing/checkout` (`AuthUser`): body `{ plan, interval }`, returns `{ url }`
  - `POST /api/billing/portal` (`AuthUser`): returns `{ url }`, for plan changes, cancelling, invoices and the card
  - `POST /api/billing/webhook` (public): reads the raw body as `Bytes` and checks the signature
  - `GET  /api/admin/billing` and `PUT /api/admin/billing/{user_unid}` (`AdminUser`): every host's plan, and give or take back a free plan
- Env vars:
  - `STRIPE_SECRET_KEY`, `STRIPE_WEBHOOK_SECRET`
  - `STRIPE_PRICE_PRO_MONTH`, `STRIPE_PRICE_PRO_YEAR`, `STRIPE_PRICE_BUSINESS_MONTH`, `STRIPE_PRICE_BUSINESS_YEAR`
  - Add them to `.env.example`.
- In Stripe, the Customer Portal is set up so customers can switch between the 4 prices with proration and cancel at the end of the period.

### Stripe setup

Without the six `STRIPE_*` vars the app runs with payments off. To turn them on in test mode:

1. Put `STRIPE_SECRET_KEY=sk_test_...` in `.env` (Stripe dashboard → Developers → API keys).
2. Run `scripts/stripe-setup.sh`. It makes the two products and four prices (amounts from `plans.rs`) and writes the `STRIPE_PRICE_*` ids into `.env`. It is safe to run again: prices are found by lookup key.
3. In the [Customer Portal settings](https://dashboard.stripe.com/test/settings/billing/portal), allow switching between the Pro and Business prices and cancelling at the end of the period, then Save.
4. Run `stripe listen --forward-to localhost:3001/api/billing/webhook`, put the `whsec_...` it prints in `STRIPE_WEBHOOK_SECRET`, and restart the backend.

For production, use a live key and `scripts/stripe-setup.sh --live`, and take the webhook secret from an endpoint registered in the dashboard instead of `stripe listen`.

### Enforcement: a single gate, entitlements read inside services

The rule is "keep the data, stop applying it".

- **The page carries the plan** (`host_settings::service::with_plan`): every page load applies the host's entitlements, so bookings, slots, leads and the public payload all see the same thing.
  - Replaced by Plan 30: Free's 25 are now included bookings, and the page never closes. Before that, `full`: Free's 25 calls this month (non-cancelled, booked this calendar month in the host's timezone) are used up. `bookable_page` then answers 404, as for a paused page, so slots, new bookings and the form close. Moving an existing call still works. The public page shows its "not taking bookings right now" card.
  - `ask_*` flags are switched off without `custom_questions`; the saved settings keep them.
  - `show_branding`: the Cadence logo under the booking card, hidden on paid plans.
  - Two calls booked at the same moment at 24 can both get through; the limit is a business limit, not a lock.
- **Reminders** (`emails/service.rs::queue_for_invitee`): `booked` and `rescheduled` take the plan's reminders and queue only those.
- **Accent colour:** `widget_settings.accent_color` is stored but nothing reads or sets it, so there is nothing to gate. The pricing page marks it "Soon".
- **Host settings save** (`host_settings/service.rs`): turning on a question that was off, without `custom_questions`, is refused with `AppError::PlanRequired` (403). Questions already on can stay on or be turned off. Settings shows a "Pro" notice and disables the switches it cannot turn on.
- **Account notices** (`emails::service::AccountNotice`, through `email_queue`):
  - `trial_ending`: queued at sign-up for 3 days before the trial ends; taken back when a Stripe subscription becomes active.
  - `host_booking_limit`: once, when the call just booked is the month's last.
  - `payment_failed`: once, when a subscription turns `past_due`.
- **Dashboard banner** (`domains/billing/plan-banner.tsx`): a failed payment, a full month, or the last 5 days of a trial.

## Frontend

Visual sketch of `/pricing`, built inside `PublicLayout`:

```
┌───────────────────────────────────────────────────────────────┐
│ logo            Pricing  ·  ◐  Log in  [Sign up]               │
├───────────────────────────────────────────────────────────────┤
│        Pick the plan that fits your day                        │
│        [ Monthly | Yearly  save 20% ]   (SegmentedControl)     │
│ ┌──────────────┐ ┌──────────────────┐ ┌──────────────┐        │
│ │ Free         │ │ Pro  ★Most popular│ │ Business     │        │
│ │ $0           │ │ $8 /mo yearly     │ │ $13 /mo      │        │
│ │ For trying   │ │ For individuals   │ │ For growing  │        │
│ │ [Start free] │ │ [Start 14-day     │ │ [Choose      │        │
│ │ ✓ 25/mo …    │ │   trial]  ✓ …     │ │  Business] ✓ │        │
│ └──────────────┘ └──────────────────┘ └──────────────┘        │
│  Compare plans (shadcn Table, grouped: Booking, Emails,        │
│  Branding, Integrations, Advanced) ✓ / – / numbers             │
│  FAQ (shadcn Accordion: trial, cancel, refunds, downgrades,    │
│  yearly billing, taxes)                                        │
│  Closing CTA band (same look as the landing page)              │
├───────────────────────────────────────────────────────────────┤
│ © · Pricing · Terms · Privacy                                  │
└───────────────────────────────────────────────────────────────┘
```

`/dashboard/billing`, built inside `AppShell` with a new "Billing" nav item using the lucide `CreditCard` icon:

```
┌──────────────┬────────────────────────────────────────────────┐
│ Overview     │ Billing                                         │
│ Bookings     │ ┌ Current plan ─────────────┐ ┌ This month ──┐  │
│ Availability │ │ Pro · trial, 9 days left  │ │ 12 / 25 calls │  │
│ Apps ▸       │ │ [Upgrade] [Manage billing]│ │ ███████░░░░░  │  │
│ Settings     │ └───────────────────────────┘ └──────────────┘  │
│ Billing  ◀   │ Plan cards (same PlanCard as /pricing, compact) │
└──────────────┴────────────────────────────────────────────────┘
```

Files:
- `frontend/src/lib/routes.ts`: add `pricing: "/pricing"` and `dashboardBilling: "/dashboard/billing"`.
- `frontend/src/lib/query-keys.ts`: add `billingPlans` and `billing`.
- `frontend/src/routes/pricing.tsx`: a loader prefetches `billingPlansQuery` for SSR; `head` sets the title and description for SEO.
- `frontend/src/domains/pricing/`:
  - `pricing-page.tsx`, `plan-card.tsx`, `compare-table.tsx`, `pricing-faq.tsx`
  - `pricing.ts`, which holds the pure logic (tested):
    - the monthly or yearly price shown
    - the savings percentage
    - the feature rows built from `Entitlements`
    - CTA resolution:
      - logged out: `/register?plan=pro&interval=year`
      - logged in on Free: POST checkout and redirect
      - on the same plan already: "Current plan"
- `frontend/src/domains/billing/`:
  - `billing-api.ts` (queryOptions and mutations)
  - `entitlements.ts`: `can(entitlements, feature)` and an upgrade target (tested)
  - `plan-gate.tsx`: a lock `Badge` and an "Upgrade to Pro" link next to a disabled control
  - `trial-banner.tsx`, shown in `AppShell` for the last 5 days of a trial and when `past_due`
- `frontend/src/routes/dashboard/billing.tsx`. After a successful checkout, `?checkout=done` shows a toast and invalidates `billing`, because the webhook may land a moment later.
- `register.tsx`: if `plan` and `interval` are in the search params, after signup send the user to `/dashboard/billing?intent=...`, which offers checkout. The trial is already running.
- `public-layout.tsx`: add a Pricing link to the header nav and the footer. `landing-page.tsx`: add a secondary "See pricing" link in the hero and the closing CTA.
- Public booking page (`domains/widget/`): render a small "Booked with Cadence" link when `show_branding` is set.
- Settings (`routes/dashboard/settings.tsx`): wrap the accent colour and booking question controls in `PlanGate`.
- Admin (`routes/admin/hosts.tsx`): add a plan/status column, and a "Comp plan" action backed by the admin billing endpoint.
- Add the shadcn `accordion` component (`pnpm dlx shadcn add accordion`).
- `/pricing` reads the user on the client with a `sessionQuery` on `/api/auth/me` that treats 401 as null, because the root guard only loads the user on protected paths.

### Pricing page UX pass

`/pricing` asks before it lists. A week sizer (calls a week, plus "I offer more than one kind of call") picks the cheapest plan whose entitlements cover the week (`recommendPlan` in `domains/pricing/pricing.ts`, read from `PlanOffer.entitlements`, never hard-coded limits). That plan gets the dusk ring, a "Fits your week" badge, the solid CTA, and a tinted column in the comparison table. Yearly prices show the monthly price crossed out and the dollars saved. The comparison table pins its plan header on wide screens and ends with the same start buttons. `/register?plan=&interval=` shows the picked plan with a link back to `/pricing`. The only new UI primitive is the shadcn `Slider`, which comes from `radix-ui`, already a dependency.

The sizer ends in the recommended plan's start button, so the answer and the action sit together on a phone. The answers live in the URL (`?interval=&calls=&kinds=`, defaults left out, `pricingSearch` drops the rest), so the back button, a shared link and register's "Change" keep them. At phone width the comparison table shows one plan's column at a time, switched with a segmented control that starts on the recommended plan. On `/dashboard/billing?plan=&interval=`, a "Finish your pick" panel offers "Subscribe to {plan}" in one click, or "Decide later" to drop the pick. During a trial, the current plan panel shows this month against Free's limit. Every checkout button on billing says "Subscribe to {plan}", and the confirmation toast says "Subscribed to {plan}."

## Milestones (each one leaves the app shippable)

1. ✅ **Catalog and trial:** the migration, `plans.rs`, `effective_plan`, `GET /api/billing/plans` and `/me`, the trial on registration, bindings, and the seed.
2. ✅ **Public pricing page:** the route, components, nav links, FAQ and comparison table. CTAs go to register.
3. ✅ **Stripe:** checkout, portal, the webhook with signature and idempotency, the `/dashboard/billing` page, and the trial banner.
4. ✅ **Enforcement:** the booking limit, reminders, public page flags, the branding badge, the `PlanGate` UI, `PlanRequired` errors, and the trial-ending, limit and payment-failed emails.
5. ✅ **Admin:** a Plans page (`/admin/plans`, `GET /api/admin/billing`) with every host's plan, per-plan counts, and give/take back a free plan (`PUT /api/admin/billing/{user_unid}`, `status = comped`). A plan Stripe bills is changed in Stripe only (409).
6. **Plan 28:** event types.
7. **Plan 29:** webhooks.

