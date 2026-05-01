# Production deployment

Production target:

- frontend: `https://wspace.cleadwine.ru`, static files in `/var/www/agent-workspace`;
- backend: Docker Compose project `agent-workspace`, API published only on `127.0.0.1:18080`;
- nginx: separate `server_name wspace.cleadwine.ru`, proxying only `/api/v1/*` and `/health` to the workspace API.

This keeps the deployment isolated from the existing `gandz` project and does not claim shared paths such as `/api` on `cleadwine.ru`.

## GitHub Actions setup

Add these repository secrets:

| Secret | Value |
| --- | --- |
| `DEPLOY_HOST` | `cleadwine.ru` or the server IP |
| `DEPLOY_USER` | SSH user with Docker and nginx permissions, currently likely `root` |
| `DEPLOY_SSH_KEY` | Private SSH key accepted by the server |
| `PRODUCTION_POSTGRES_PASSWORD` | Strong password for the production workspace Postgres user |

The workflow is `.github/workflows/deploy-production.yml`. It runs on pushes to `master` and can also be started manually with `workflow_dispatch`.

For GitHub OAuth in production, add these values to `/opt/agent-workspace/.env` on the server:

- `GITHUB_CLIENT_ID`
- `GITHUB_CLIENT_SECRET`
- `GITHUB_OAUTH_REDIRECT_URI=https://wspace.cleadwine.ru/api/v1/auth/github/callback`
- `GITHUB_OAUTH_SUCCESS_REDIRECT_PATH=/`

## First server setup

Point DNS `A`/`AAAA` records for `wspace.cleadwine.ru` to the server.

The first deploy creates `/opt/agent-workspace/.env`, installs `/etc/nginx/sites-available/wspace.cleadwine.ru` only when it does not exist, enables it, builds the backend container, starts Postgres, uploads the frontend, validates nginx config, and reloads nginx. Later deploys preserve the persistent server-side `.env` and existing nginx file so database credentials and Certbot-managed HTTPS settings are not overwritten.

After DNS is live, issue TLS for the new subdomain:

```bash
certbot --nginx -d wspace.cleadwine.ru
```

## Manual deploy from this machine

The local SSH alias `cleadwine` can be used directly:

```powershell
.\scripts\deploy-production.ps1
```

Optional parameters:

```powershell
.\scripts\deploy-production.ps1 -SshHost cleadwine -ApiHostPort 18080
```

The script generates a Postgres password on first run if `/opt/agent-workspace/.env` does not exist on the server. Later deploys keep that persistent server-side `.env`.
