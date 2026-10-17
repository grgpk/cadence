# Plan 30: pricing promises

Status: in progress. Free no longer closes the page, and the price lock is
enforced in the Stripe setup and the webhook. The promises on `/pricing` come
next.

## Goal

Give a solo host a reason to pick Cadence over the big schedulers. Make three
promises on `/pricing`, and make each one true in code:

- **Nobody is turned away.** Past Free's 25 included bookings, the page stays
  open.
- **Your price stays.** It never goes up while you stay subscribed.
- **Nothing is deleted.** Changing plans or cancelling keeps every setting.

The site names no competitor. It states the advantages plainly.

## Gap

Research on 2026-10-17, for one person:

| | Cadence before | Calendly | Cal.com |
|---|---|---|---|
| Free bookings | 25, then the page 404s | unlimited | unlimited |
| Free reminders | 24h email | none | email + SMS |
| Remove the logo | $8 (Pro, yearly) | $10 yearly / $12 monthly | $12 a user (Teams) |
| Several event types | Business $13 | Standard $10 | Free |

Sources:
- [calendly.com/pricing](https://calendly.com/pricing)
- [cal.com/pricing](https://cal.com/pricing)
- [Zoho on Calendly's plans](https://www.zoho.com/bookings/explore/calendly-pricing-plans.html), for monthly rates and Free reminders

The Free cap was the one place we were worse than both. The cheapest logo
removal and reminders on Free are where we are better.

## Data model

There is no migration.

`Entitlements.max_bookings_per_month` is renamed to
`included_bookings_per_month`, because past it nothing is refused. The
wire fields `BookingPage.full` and `HostPage.full` are removed.

## Enforcement

- **Bookings:** `bookable_page` checks only `accepting_bookings`.
  `billing::notice_if_past_included` still queues `host_booking_limit` once a
  month, when the call just booked is the last one included. The email now
  reads "You've had 25 bookings this month", says the page stays open, and
  links to the plans.
- **Price lock:**
  - `scripts/stripe-setup.sh` never reuses a price whose amount changed. It
    makes a new one with `transfer_lookup_key`, and Stripe keeps existing
    subscriptions on the old one.
  - Every price, and the one `.env` pointed at before, gets `metadata.plan`.
  - `PriceIds::plan_of` reads that metadata and the price's interval when the
    price id is not one of today's. A subscriber on an older price keeps their
    plan.
  - Tested against Stripe test mode: run on unchanged amounts, the script
    reused the four keyed prices and tagged all eight.
- **Nothing is deleted:** this was already the rule from Plan 27. The billing
  status line now says it when a plan is cancelled or has ended.

## UI

- **Dashboard banner:** "You've had N bookings this month", saying the page
  stays open, with a "See plans" button.
- **Billing usage row:** "N of 25 included". Past 25 the bar turns warning,
  not destructive.
- **Pricing:**
  - the compare table says "25 included" and gains a row "Nobody is turned
    away past it";
  - the week sizer says "past the 25 Free includes".
- **Promises list:** beside the week sizer, three promises with one muted line
  each. It stacks under the sizer on a phone.

```
┌──────────────────────────────────────┬──────────────────────────────┐
│ How busy is your week?               │ ✓ Nobody is turned away      │
│ 12 calls a week ●━━━━○────           │ 🔒 Your price stays          │
│ Pro fits: … [Start 14-day Pro trial] │ ⟲ Nothing is deleted         │
└──────────────────────────────────────┴──────────────────────────────┘
```

## Tests

- `host_booking_http::a_free_page_keeps_taking_calls_past_the_included_month`:
  - the 25th call queues one notice;
  - the 26th call books;
  - slots still answer.
- `billing::stripe::tests::an_older_price_keeps_its_plan_by_metadata`.
- Frontend unit tests cover:
  - the new fit and allowance copy;
  - the "25 included" row;
  - the cancelled-plan status line.

## Definition of done

- [x] Free never refuses a booking; the host is told once a month.
- [x] The price lock is enforced in the setup script and in the webhook.
- [ ] The promises are on `/pricing`, in the cards and in the FAQ.
- [ ] Rust and frontend checks pass; screenshots at 1440 and 390.
