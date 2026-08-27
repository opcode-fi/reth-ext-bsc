#!/usr/bin/env bash
# Build the node from a reth-bsc checkout and (re)start it.
#
# Always rebuilds. A plain `docker compose restart` ships the OLD image, which is a very
# easy way to spend an hour debugging a fix that was never deployed.
set -euo pipefail

REPO="${RETH_BSC_REPO:-$HOME/reth-bsc}"
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

[ -d "$REPO" ] || { echo "no reth-bsc checkout at $REPO (set RETH_BSC_REPO)" >&2; exit 1; }

echo "building reth-bsc:local from $REPO"
docker build -t reth-bsc:local "$REPO"

echo "starting"
docker compose -f "$HERE/docker-compose.yml" up -d

# Never `down --remove-orphans` here: this compose project shares a host with other
# services, and it will take them with it.
docker compose -f "$HERE/docker-compose.yml" ps
