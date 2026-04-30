param(
    [string]$SshHost = "cleadwine",
    [string]$RemoteAppDir = "/opt/agent-workspace",
    [string]$RemoteWebDir = "/var/www/agent-workspace",
    [string]$PostgresPassword = "",
    [int]$ApiHostPort = 18080
)

$ErrorActionPreference = "Stop"

function Run($Command) {
    Write-Host "> $Command"
    Invoke-Expression $Command
}

if ([string]::IsNullOrWhiteSpace($PostgresPassword)) {
    $PostgresPassword = -join ((48..57 + 65..90 + 97..122) | Get-Random -Count 32 | ForEach-Object {[char]$_})
}

Run "npm --prefix web ci"
Run "npm --prefix web run build"

$archive = Join-Path $env:TEMP "agent-workspace-deploy.tar"
if (Test-Path $archive) {
    Remove-Item -LiteralPath $archive
}

Run "git archive --format=tar --output `"$archive`" HEAD"

ssh $SshHost "mkdir -p '$RemoteAppDir/releases/current' '$RemoteWebDir'"
scp $archive "${SshHost}:/tmp/agent-workspace-deploy.tar"
ssh $SshHost "rm -rf '$RemoteAppDir/releases/current' && mkdir -p '$RemoteAppDir/releases/current' && tar --warning=no-timestamp -xf /tmp/agent-workspace-deploy.tar -C '$RemoteAppDir/releases/current' && rm /tmp/agent-workspace-deploy.tar"

scp -r web/dist/* "${SshHost}:${RemoteWebDir}/"
scp deploy/nginx/wspace.cleadwine.ru.conf "${SshHost}:/tmp/wspace.cleadwine.ru.conf"

$remote = @"
set -euo pipefail
cd '$RemoteAppDir/releases/current'
if [ ! -f '$RemoteAppDir/.env' ]; then
  cat > '$RemoteAppDir/.env' <<EOF
POSTGRES_DB=agent_workspace
POSTGRES_USER=agent_workspace
POSTGRES_PASSWORD=$PostgresPassword
API_HOST_PORT=$ApiHostPort
EOF
fi
docker compose --env-file '$RemoteAppDir/.env' -f deploy/production/docker-compose.yml up -d --build
if [ ! -f /etc/nginx/sites-available/wspace.cleadwine.ru ]; then
  install -o root -g root -m 0644 /tmp/wspace.cleadwine.ru.conf /etc/nginx/sites-available/wspace.cleadwine.ru
  ln -sfn /etc/nginx/sites-available/wspace.cleadwine.ru /etc/nginx/sites-enabled/wspace.cleadwine.ru
fi
chown -R www-data:www-data '$RemoteWebDir'
nginx -t
systemctl reload nginx
"@

$encoded = [Convert]::ToBase64String([Text.Encoding]::UTF8.GetBytes($remote))
ssh $SshHost "echo $encoded | base64 -d | bash"
