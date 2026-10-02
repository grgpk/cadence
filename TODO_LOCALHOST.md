# TODO on your own

This file lists the local credentials and provider setup required to run
Cadence locally. Never commit `.env`, OAuth client secrets, Resend API keys,
JWT secrets, or database passwords.

## 1. Create local environment

```sh
cp .env.example .env
```

Fill these values in `.env`:

```dotenv
# Database
POSTGRES_USER=postgres
POSTGRES_PASSWORD=your-local-postgres-password
POSTGRES_DATABASE=cadence
DATABASE_URL=postgres://postgres:your-local-postgres-password@127.0.0.1/cadence

# Auth. Generate a different value per environment.
JWT_SECRET=generate-a-long-random-secret
APP_ENV=local
ENVIRONMENT=local
CORS_ORIGINS=http://localhost:3000

# Frontend -> backend during local development
VITE_API_URL=http://localhost:3001

# Google Calendar OAuth
GOOGLE_CLIENT_ID=...
GOOGLE_CLIENT_SECRET=...
GOOGLE_REDIRECT_URI=http://localhost:3000/admin/google-oauth-callback

# Email sender and Resend SMTP
EMAIL_FROM=Cadence <your-verified-sender@example.com>
RESEND_API_KEY=re_...
RESEND_SMTP_HOST=smtp.resend.com
RESEND_SMTP_PORT=587
RESEND_SMTP_USERNAME=resend
```

Use a real local Postgres instance. Then run:

```sh
pnpm install
./reset_db.sh --yes
pnpm run dev
```

Open:

- App: `http://localhost:3000`
- Admin: `http://localhost:3000/admin`
- Backend health: `http://localhost:3001/health`

Development seed users use password `password`. Use
`admin@example.com` for admin and Google Calendar tests.

## 2. Google Calendar OAuth

In Google Cloud Console:

1. Create or select the Cadence project.
2. Enable the Google Calendar API.
3. Configure the OAuth consent screen.
4. Add the Google account used for testing as a test user if the app is not
   published.
5. Create an OAuth client of type **Web application**.
6. Add this exact local authorized redirect URI:

   ```text
   http://localhost:3000/admin/google-oauth-callback
   ```

7. Copy the client ID and client secret into `.env`.

The app requests the `https://www.googleapis.com/auth/calendar.events` scope.
After starting Cadence, log in as the admin user, open `/admin`, and click
**Connect Google Calendar**. Google must redirect back to the exact URI above.

For production, add this second redirect URI to the same OAuth client or use a
separate production client:

```text
https://cadence.rustify.app/admin/google-oauth-callback
```

Common failure: `redirect_uri_mismatch`. Fix by comparing the URI character by
character. `localhost` and `127.0.0.1` are different OAuth origins.

## 3. Resend and email delivery

Cadence does not use a generic `SMTP_PASSWORD`. The email worker sends through
Resend SMTP using the Resend API key as the SMTP password:

| Variable | Value |
| --- | --- |
| `EMAIL_FROM` | Verified sender shown to recipients |
| `RESEND_API_KEY` | Resend API token, starts with `re_` |
| `RESEND_SMTP_HOST` | `smtp.resend.com` |
| `RESEND_SMTP_PORT` | `587` |
| `RESEND_SMTP_USERNAME` | `resend` |

In Resend:

1. Create an API key with sending permission.
2. Verify the sending domain or sender address.
3. Put that exact verified address in `EMAIL_FROM`.
4. Book a test call and check the worker logs.

Booking emails are queued in Postgres and processed by the backend worker every
60 seconds. Templates currently include:

- booking confirmation
- 24-hour reminder
- 2-hour reminder
- 30-minute reminder

### Local-only email option

If Resend is not ready, run a local SMTP inbox such as Mailpit:

```sh
docker run --rm --name cadence-mailpit \
  -p 1025:1025 -p 8025:8025 \
  axllent/mailpit
```

For Mailpit, remove or comment out `RESEND_SMTP_HOST`, `RESEND_SMTP_PORT`,
`RESEND_SMTP_USERNAME`, and `RESEND_API_KEY`. The worker then uses
`localhost:1025`. Open `http://localhost:8025` to inspect emails.

## 4. Local verification checklist

- [ ] `.env` exists and is not tracked by Git.
- [ ] Postgres is running.
- [ ] Migrations and seed data complete.
- [ ] `http://localhost:3000/login` works.
- [ ] Admin page opens.
- [ ] Google Calendar connects from `/admin`.
- [ ] Calendar events load after connection.
- [ ] Available slots exclude Google Calendar events.
- [ ] A test booking creates a Google Calendar event and Google Meet link.
- [ ] Booking confirmation appears in Resend or Mailpit.
- [ ] Reminder rows are present in the email queue.

Useful checks:

```sh
curl -i http://localhost:3001/health
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
pnpm --dir frontend typecheck
pnpm --dir frontend lint
pnpm --dir frontend knip
```

If Google Calendar fails, check the backend logs first. If email fails, check
the email worker logs and the `email_queue` rows before changing frontend code.
