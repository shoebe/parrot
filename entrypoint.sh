#!/bin/sh
set -e

echo "Updating yt-dlp on startup..."
yt-dlp -U || true

# Start busybox crond in the background for daily updates
crond -b -l 2

exec "$@"

