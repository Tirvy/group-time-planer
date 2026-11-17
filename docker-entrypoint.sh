#!/bin/sh
# Container entrypoint.
#
# - Honors an explicit APP_BIND_ADDR (docker compose) and otherwise binds to
#   the PORT injected by PaaS platforms such as Railway, defaulting to 8080.
# - Named volumes (Railway, Docker) are mounted as root. When started as root,
#   hand the database directory to the unprivileged `app` user and drop
#   privileges before exec'ing the server.
set -eu

: "${DATABASE_PATH:=/app/data/app.db}"
[ -n "${APP_BIND_ADDR:-}" ] || export APP_BIND_ADDR="0.0.0.0:${PORT:-8080}"

if [ "$(id -u)" = "0" ]; then
    data_dir=$(dirname "$DATABASE_PATH")
    mkdir -p "$data_dir"
    chown -R app:app "$data_dir"
    exec setpriv --reuid=app --regid=app --init-groups raid-scheduler-backend
fi

exec raid-scheduler-backend
