# Plan 38: personal link previews, keeping hosts who cancel, two small gaps

Status: done.
- Rust: fmt, clippy and every test pass.
- Frontend: build, typecheck, lint, knip and tests pass.

Not yet checked in the browser or against Stripe test mode: the dev servers
were not running.

## Goal

- **Link previews.** A booking link pasted into a chat or an email shows
  the host's own card, not one generic picture.
- **Cancelling.** Hosts who cancel say why. They are offered what fits the
  reason: paying yearly, a free month's pause, or Free with their page
  kept. The reasons show in Admin.
- **The two gaps left by Plan 37:**
  - the weekly email is told apart by host, not by address;
  - a refund takes back the referral credits its invoice earned.

## Schema

There are no ALTERs. Migrations from `0017` on exist only on this branch
(production has up to `0016`), so the schema went into their CREATE
statements. Databases are rebuilt with `./reset_db.sh --yes`.

- **`0017_create_subscriptions.sql`:**
  - `subscriptions.stripe_checkout_session_id` (Plan 37) and
    `subscriptions.paused_until`;
  - the new table `cancellation_feedback`, with reason, note, outcome and
    when.
- **`0021_create_referrals.sql`:** `referred_invoice_id` and `revoked_at`.
- **`0023` is gone:** its column is in 0017.

## Link previews

- `GET /api/hosts/{slug}/preview.png` returns a 1200×630 card. It shows:
  - the host's name, and their accent as a bar (when the plan shows it);
  - up to three kinds of call with lengths, then "+N more";
  - the link, and "Cadence" on Free.
- It is cached for an hour, and a paused or unknown page is 404.
- `host_settings/preview.rs` draws the card as SVG and turns it into PNG
  with **`resvg`**.
  - This is a new dependency: chat apps need PNG, and resvg is pure Rust,
    so the Docker build is unchanged.
  - It is its own module because drawing is neither a business rule nor
    SQL.
  - Host text is XML-escaped and cut with "…", since SVG does not wrap.
  - Fonts: Hanken Grotesk Regular and Bold TTF, the site's family, under
    the OFL (licence included), in `host_settings/fonts/` next to the code
    that embeds them, as `emails/logo.png` is. resvg cannot read the site's
    woff2 files.
- `book/$slug` and `book/$slug/$type` add `og:image`, its size and
  `twitter:image` (`previewMeta`). TanStack keeps the deepest route's tag
  per `property`, so the root's generic picture is replaced.
- To look at sample cards:
  `CADENCE_PREVIEW_DIR=/some/dir cargo test -p cadence-backend --lib link_previews -- --ignored`.

```
┌──────────────────────────────────────────────────────────┐
│▌  Book a call with                                       │
│▌  Nino Tbilisi                                           │
│▌  Intro call                                     30 min  │
│▌  Deep dive                                 1 h 30 min   │
│▌  bookcadence.app/book/nino                     Cadence  │
└──────────────────────────────────────────────────────────┘
```

## Keeping hosts who cancel

**Billing:** "Current plan" has "Cancel plan" for an active paid plan, and
"Keep my plan" once one is set to end. "Cancel plan" opens "Before you
go":
- six reasons, and "What would have kept you?";
- **the offer that fits** (`leaveOffer`):
  - leaving over price, as a monthly payer: pay yearly, through the
    existing switch;
  - too few calls, or a break: **pause free for a month**, once per host.
    The plan stays on and nothing is charged until billing restarts by
    itself;
- always: "Cancel at period end". The page stays open on Free.

**Endpoints:**
- `POST /api/billing/cancel {reason, note?}`: `cancel_at_period_end` with
  Stripe's `cancellation_details` (reason mapped to Stripe's own values);
- `POST /api/billing/pause {reason, note?}`:
  `pause_collection[behavior]=void` and `resumes_at` in 30 days; 409 the
  second time;
- `POST /api/billing/resume`: takes the cancel back;
- `POST /api/billing/yearly-instead {reason, note?}`: keeps why, then the
  page goes to the yearly switch;
- each keeps a `cancellation_feedback` row and shows the change at once
  (`mark_change`); the webhook confirms it.

**The rest:**
- The webhook mirrors `pause_collection.resumes_at` into `paused_until`.
  `BillingSummary` has `paused_until` and `paused_before`, and the status
  line says "paused until …".
- **Admin, Plans:** "Why hosts cancel" shows counts per reason and the
  latest notes (`GET /api/admin/billing/cancellations`, admins only).

## The two gaps

- **Weekly email:** its payload carries `host_unid`. "Already queued this
  week" matches the host, and the address only for rows queued before. The
  test changes the host's email between runs and still sees one email.
- **Refunds:**
  - Credits now remember the invoice that earned them.
  - `charge.refunded`, when fully refunded, finds that invoice: on the
    charge for older API versions, or through
    `GET /v1/invoice_payments?payment[payment_intent]=…`. It then takes
    $10 back from each side that was credited, keyed
    `referral-<referred>-<side>-revoked`.
  - The event is recorded only after that worked, so a Stripe failure
    fails the event and Stripe sends it again. It changes no plan, so
    nothing waits on it.
  - **Setup:** `stripe listen` and the production endpoint need
    `charge.refunded` too (in `scripts/stripe-setup.sh`'s notes and Plan
    27).

## Tests

- **Unit:**
  - the preview SVG escapes host text, cuts long names, counts extra calls
    and renders a PNG;
  - `leaveOffer` and `trialEnded`.
- **HTTP:**
  - the preview endpoint (PNG, 404);
  - cancel, keep, pause once, and the admin-only reasons;
  - a refund takes both credits back once, and another invoice's refund
    changes nothing.
