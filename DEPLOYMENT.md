# Deployment

Cadence production runs on the `shared-apps` server behind Nginx:

```text
https://cadence.rustify.app
```

Deployment is GitHub Actions driven. Pushes to `main` run quality checks, then
build and deploy the production containers.

## Production layout

```text
Cloudflare DNS
      |
      v
Nginx :80/:443
  |             |
  v             v
frontend     backend
:5104        :5105
                  |
                  v
              Postgres
```

- Server: `shared-apps` SSH alias.
- Server IP: `23.88.45.210`.
- App directory: `/root/cadence`.
- Frontend: `127.0.0.1:5104`.
- Backend: `127.0.0.1:5105`.
- Postgres data: Docker volume `cadence_cadence_db`.
- Nginx config: `/etc/nginx/conf.d/cadence.rustify.app.conf`.
- TLS certificate: `/etc/letsencrypt/live/cadence.rustify.app/`.

The production `.env` lives only at `/root/cadence/.env`. It is ignored by Git
and must never be copied into the repository or GitHub logs.

## GitHub configuration

Repository: `grgpk/cadence`.

Required GitHub secret:

| Type | Name | Content |
| --- | --- | --- |
| Secret | `SSH_PRIVATE_KEY` | Private key allowed to SSH as `root` on shared-apps |

Required GitHub variables:

| Name | Value |
| --- | --- |
| `SERVER_IP` | `23.88.45.210` |
| `LETSENCRYPT_EMAIL` | `contact@rustify.rs` |

Inspect names without exposing secret values:

```sh
gh secret list
gh variable list
```

Set or update values:

```sh
gh variable set SERVER_IP --body '23.88.45.210'
gh variable set LETSENCRYPT_EMAIL --body 'contact@rustify.rs'
gh secret set SSH_PRIVATE_KEY < ~/.ssh/hetzner-mac
```

Do not add Google, Resend, JWT, or database secrets to GitHub unless the
workflow explicitly needs them. Runtime application secrets belong in the
server `.env`.

## DNS and TLS

The DNS record must point to the shared-apps server:

```text
A cadence.rustify.app -> 23.88.45.210
```

The deploy workflow installs an HTTP bootstrap Nginx config, requests the
Let’s Encrypt certificate with Certbot, then switches to the HTTPS config.
After the first successful certificate request, later deploys reuse the
existing certificate and Nginx config.

Check DNS and HTTPS:

```sh
dig +short A cadence.rustify.app
curl -I https://cadence.rustify.app
```

## Normal deploy

Preferred flow:

```sh
git checkout main
git pull --ff-only origin main
git status --short
git push origin main
```

GitHub Actions then runs:

1. Rust formatting and Clippy.
2. Binding export test.
3. Frontend build, lint, and Knip.
4. Remote `git reset --hard origin/main` in `/root/cadence`.
5. Docker image build.
6. `docker compose up -d`.
7. Nginx and TLS setup.
8. Backend, frontend, and public smoke tests.

Watch the workflow:

```sh
gh run list --workflow deploy.yml --limit 5
gh run watch <RUN_ID>
```

Manual deploy trigger:

```sh
gh workflow run deploy.yml --ref main
```

## Server checks

```sh
ssh shared-apps
cd /root/cadence
docker compose -f docker-compose.prod.yml ps
docker compose -f docker-compose.prod.yml logs --tail=100 backend frontend
curl -fsS http://127.0.0.1:5105/health
curl -fsS http://127.0.0.1:5104/
nginx -t
systemctl status nginx
```

Public checks:

```sh
curl -fsS https://cadence.rustify.app/health
curl -I https://cadence.rustify.app
```

## Updating production `.env`

SSH to the production server. Do not commit or print the file:

```sh
ssh shared-apps
cd /root/cadence
chmod 600 .env
${EDITOR:-vi} .env
docker compose -f docker-compose.prod.yml up -d --force-recreate backend frontend
docker compose -f docker-compose.prod.yml logs --tail=100 backend frontend
```

Important production values:

```dotenv
GOOGLE_REDIRECT_URI=https://cadence.rustify.app/admin/google-oauth-callback
CORS_ORIGINS=https://cadence.rustify.app
VITE_API_URL=
```

The production Google OAuth client must contain this exact redirect URI:

```text
https://cadence.rustify.app/admin/google-oauth-callback
```

## Rollback

Deployment follows `origin/main` exactly. To recover from a bad commit, revert
the commit on `main` and push the revert. The workflow then deploys the
reverted state:

```sh
git revert <BAD_COMMIT>
git push origin main
```

For an emergency server-only rollback, inspect available images first:

```sh
ssh shared-apps 'docker image ls cadence-backend cadence-frontend'
```

Do not run `git reset --hard` locally to recover work. The deployment workflow
uses its hard reset only inside `/root/cadence`, after GitHub has accepted the
commit on `main`.

## Database and migration rules

- Never edit a migration that may already have run.
- Add the next numeric migration instead.
- Production backend runs migrations at startup.
- Back up or snapshot the Postgres volume before risky schema work.
- Check backend logs after every migration deploy.

## Deployment definition of done

- [ ] GitHub quality job passes.
- [ ] Deploy job passes.
- [ ] `cadence.rustify.app` resolves.
- [ ] HTTPS certificate is valid.
- [ ] Backend health returns `200`.
- [ ] Frontend returns `200`.
- [ ] Admin login works.
- [ ] Google Calendar remains connected.
- [ ] Test booking creates the calendar event and email queue rows.
