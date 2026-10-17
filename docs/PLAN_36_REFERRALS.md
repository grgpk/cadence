# Plan 36: give a month, get a month

Status: done.
- Rust: fmt, clippy and every test pass.
- Frontend: build, typecheck, lint, knip and tests pass.

Checked in the browser:
- the invite box on Billing;
- "Make your own booking page" in a Free booking page's footer, linking to
  `/register?ref=<address>`;
- the invitation note on sign-up.

Checked in Stripe test mode (2026-10-17), with two hosts paying through
real Checkout:
- the invited host got a −$10 balance transaction when they paid;
- the inviter's credit waited while they were on the trial, then arrived
  when they paid;
- a resent `customer.subscription.created` event credited nobody again.

## Goal

Hosts bring hosts. Each host has an invite link. When someone signs up with
it and starts paying, both get a month of Pro ($10) off their next bills.
Every Free booking page carries the host's invite, so the pages people
already share do the inviting.

## Gap

There was no way to reward word of mouth. The Cadence logo on Free pages
went to the home page and counted nobody.

## Data model

Migration `0021_create_referrals.sql`, table `referrals`:
- `referred_unid` (PK), `referrer_unid`, `created_at`;
- `referred_credited_at` and `referrer_credited_at`, each set once;
- the check `referred_unid <> referrer_unid`.

The invite code is the inviter's booking-page address (`public_slug`), so
there is no new code to store. Renaming the address later keeps
invitations already made: they are stored by account.

## API

- `POST /api/auth/register` takes `referred_by?`, recorded in the
  sign-up transaction. An unknown address is ignored.
- `GET /api/billing/referrals` → `Referrals { code, invited, paying,
  credit_cents }`.

## Flow

An applied `customer.subscription.*` event with status `active` runs
`credit_referrals` inside the webhook transaction:
1. The paying host came by invitation and was not credited: credit them,
   and credit the inviter too if the inviter pays (has an active Stripe
   subscription).
2. Invitations this host made that are paid but not yet credited to them:
   credit each now.

A credit is `POST /v1/customers/{id}/balance_transactions` with a negative
amount and an `Idempotency-Key` of `referral-<referred>-<side>`. If Stripe
fails, the transaction rolls back and Stripe retries the event; the key
keeps a retried credit from landing twice.

```
Billing
┌ Give a month, get a month ────────────────────────────┐
│ When someone signs up with your link and starts       │
│ paying, you each get $10 off, a month of Pro.         │
│ [ https://…/register?ref=ada-lovelace ]  [Copy link]  │
│ 2 signed up with it, 1 of them paying.                │
└───────────────────────────────────────────────────────┘

Free booking page footer
 Cadence   Make your own booking page        Terms  Privacy
```

## Abuse

An invitation pays out only after a real payment, and each side gets
credit no larger than one month of Pro. Paying for a second account to earn
credit costs at least as much as it earns.

## Tests

- HTTP: `an_invitation_credits_both_hosts_a_month_once_each_pays`. The
  invited host pays and is credited; the inviter on trial waits, then is
  credited on paying. Repeated events credit nobody again. An unknown
  `referred_by` still signs up.
- Vitest: `signupSearch` keeps a valid `ref` and drops anything else.

## Definition of done

An invite link exists on Billing and in every Free booking page. Both sides
are credited exactly once, after the invited host's first payment.
