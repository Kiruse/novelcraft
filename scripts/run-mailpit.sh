#!/usr/bin/env bash
set -euo pipefail

# Start Mailpit — a dev SMTP server that captures mail in a web UI.
# Set PROD=true for canonical ports (requires root / port forwarding).
# Container is ephemeral (--rm): auto-deleted on stop, no persistence.

NAME="novelcraft-mail"
IMAGE="docker.io/axllent/mailpit:latest"
PROD="${PROD:-false}"

if [ "$PROD" = "true" ]; then
  SMTP_PORT=25
  WEB_PORT=80
else
  SMTP_PORT=10025
  WEB_PORT=8025
fi

# Remove stale container if any (leftover from a crash)
podman rm -f "$NAME" 2>/dev/null || true

podman run -d \
  --name "$NAME" \
  --rm \
  -p "${SMTP_PORT}:1025" \
  -p "${WEB_PORT}:8025" \
  "$IMAGE"

echo "✓ Mailpit running ($([ "$PROD" = true ] && echo production || echo dev) ports)"
echo "  Web UI: http://localhost:${WEB_PORT}"
echo "  SMTP:   localhost:${SMTP_PORT}"
