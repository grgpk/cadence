# Deployment

Cadence runs at `https://bookcadence.app` on `37.27.157.169`.

## One-time setup

GitHub Actions configuration:

| Type | Name |
| --- | --- |
| Secret | `SSH_PRIVATE_KEY` |
| Secret | `PROD_DOCKER_USERNAME` |
| Secret | `PROD_DOCKER_PASSWORD` |
| Variable | `SERVER_IP=37.27.157.169` |
| Variable | `PROD_DOCKER_REPOSITORY=cadence` |
| Variable | `LETSENCRYPT_EMAIL=contact@rustify.rs` |

Current Cadence GitHub setup already has `SSH_PRIVATE_KEY`, `SERVER_IP`, and
`LETSENCRYPT_EMAIL`. Add the two Docker secrets and repository variable before
first image deploy.

Server prerequisites, needed on any new server before the first deploy:

- The public half of `SSH_PRIVATE_KEY` is in `/root/.ssh/authorized_keys`
  (`ssh-keygen -y -f <private-key>` prints it). Without it the workflow fails
  with `Permission denied (publickey,password)`.
- Docker with the compose plugin, Nginx, and certbot are installed.
- Port 80 is reachable on `bookcadence.app` so certbot can issue the
  certificate.

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

DNS: `A bookcadence.app -> 37.27.157.169`.

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
ssh root@37.27.157.169 'cd /root/cadence && docker compose --env-file .env -f docker-compose.prod.yml ps'
curl -fsS https://bookcadence.app/health
```

Production migrations run when backend starts. Add new migration files; never
edit one already applied. Revert bad deploy by reverting commit on `main`.

## Backups

`scripts/backup-db.sh` runs on the server. It dumps the `db` container with
`pg_dump --format=custom` into `/root/cadence-backups/` (root-only, UTC
timestamped) and deletes dumps older than 14 days. Deploy copies it to
`/root/cadence/scripts/`. Override with `BACKUP_DIR` and `BACKUP_KEEP_DAYS`.

Schedule it once per server, nightly at 03:15 server time:

```sh
ssh root@37.27.157.169 "echo '15 3 * * * root /root/cadence/scripts/backup-db.sh >> /var/log/cadence-backup.log 2>&1' > /etc/cron.d/cadence-backup"
```

Run one now, or pull the newest dump to `./backups/` (gitignored):

```sh
ssh root@37.27.157.169 /root/cadence/scripts/backup-db.sh
./scripts/pull-backup.sh
```

Check a dump locally by restoring it into a scratch database:

```sh
createdb -h 127.0.0.1 -U postgres cadence_restore_test
pg_restore -h 127.0.0.1 -U postgres -d cadence_restore_test --no-owner backups/cadence_<timestamp>.dump
```

Restore production only on purpose. This replaces existing objects. Containers
are found by compose labels because `docker compose` on the server needs the
deploy-time image variables:

```sh
ssh root@37.27.157.169 'docker stop "$(docker ps -aq --filter label=com.docker.compose.project=cadence --filter label=com.docker.compose.service=backend)"'
ssh root@37.27.157.169 'docker exec -i "$(docker ps -q --filter label=com.docker.compose.project=cadence --filter label=com.docker.compose.service=db)" sh -c '\''pg_restore -U "$POSTGRES_USER" -d "$POSTGRES_DB" --clean --if-exists --no-owner'\''' < backups/cadence_<timestamp>.dump
ssh root@37.27.157.169 'docker start "$(docker ps -aq --filter label=com.docker.compose.project=cadence --filter label=com.docker.compose.service=backend)"'
```
