#!/usr/bin/env bash
# One command to start the planner locally:  ./scripts/dev.sh
set -u
cd "$(dirname "$0")/.."

mkdir -p /tmp/target

running() { docker ps --format '{{.Names}}' | grep -qx planner-db; }
exists()  { docker ps -a --format '{{.Names}}' | grep -qx planner-db; }

if ! running; then
  echo "Starting Postgres..."
  if exists && ! docker start planner-db >/dev/null 2>&1; then
    echo "Old container is broken; recreating it."
    docker rm -f planner-db >/dev/null 2>&1
  fi
  if ! running; then
    docker run -d --name planner-db --restart unless-stopped \
      -e POSTGRES_PASSWORD=dev -e POSTGRES_DB=planner \
      -v planner-pgdata:/var/lib/postgresql/data \
      -p 5432:5432 postgres:16 >/dev/null
  fi
fi

echo "Waiting for Postgres..."
for _ in $(seq 1 30); do
  docker exec planner-db pg_isready -U postgres >/dev/null 2>&1 && break
  sleep 1
done
docker exec planner-db pg_isready -U postgres || { echo "Postgres did not start"; exit 1; }

export DATABASE_URL=postgres://postgres:dev@localhost:5432/planner
exec cargo leptos watch
