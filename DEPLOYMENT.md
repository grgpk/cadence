# Deployment

Cadence runs at `https://cadence.rustify.app` on `23.88.45.210`.

## One-time setup

GitHub Actions configuration:

| Type | Name |
| --- | --- |
| Secret | `SSH_PRIVATE_KEY` |
| Secret | `PROD_DOCKER_USERNAME` |
| Secret | `PROD_DOCKER_PASSWORD` |
| Variable | `SERVER_IP=23.88.45.210` |
| Variable | `PROD_DOCKER_REPOSITORY=cadence` |
| Variable | `LETSENCRYPT_EMAIL=contact@rustify.rs` |

Current Cadence GitHub setup already has `SSH_PRIVATE_KEY`, `SERVER_IP`, and
`LETSENCRYPT_EMAIL`. Add the two Docker secrets and repository variable before
first image deploy.

Application runtime secrets are separate and stay on the server:

| Group | Variables in `/root/cadence/.env` |
| --- | --- |
| Database | `POSTGRES_USER`, `POSTGRES_PASSWORD`, `POSTGRES_DATABASE`, `DATABASE_URL` |
| Auth | `JWT_SECRET`, `APP_ENV`, `ENVIRONMENT`, `CORS_ORIGINS` |
| Google Calendar | `GOOGLE_CLIENT_ID`, `GOOGLE_CLIENT_SECRET`, `GOOGLE_REDIRECT_URI` |
| Resend SMTP | `EMAIL_FROM`, `RESEND_API_KEY`, `RESEND_SMTP_HOST`, `RESEND_SMTP_PORT`, `RESEND_SMTP_USERNAME` |

Create server runtime env. Never commit it:

```sh
cp .env.prod.example .env.prod
# Fill values, then:
./__setup_prod_server_env.sh
```

Runtime secrets stay in `/root/cadence/.env`: Postgres credentials, `JWT_SECRET`,
Google OAuth client (`GOOGLE_CLIENT_ID`, `GOOGLE_CLIENT_SECRET`, redirect URI),
and Resend SMTP (`RESEND_API_KEY`, `RESEND_SMTP_*`, `EMAIL_FROM`). Docker
registry credentials stay in GitHub.

DNS: `A cadence.rustify.app -> 23.88.45.210`.

## Deploy

```sh
git checkout main
git pull --ff-only origin main
./deploy_prod.sh
```

Script checks quality, tags the commit, and starts `.github/workflows/deploy.yml`.
Workflow builds and pushes Docker images, SSHes to server, pulls tagged images,
starts Postgres/backend/frontend, configures Nginx/TLS, then smoke-tests.

Skip local checks only when intentional:

```sh
./deploy_prod.sh --skip-quality
```

## Inspect

```sh
gh run list --workflow deploy.yml --limit 5
ssh root@23.88.45.210 'cd /root/cadence && docker compose --env-file .env -f docker-compose.prod.yml ps'
curl -fsS https://cadence.rustify.app/health
```

Production migrations run when backend starts. Add new migration files; never
edit one already applied. Revert bad deploy by reverting commit on `main`.
