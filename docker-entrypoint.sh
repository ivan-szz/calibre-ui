#!/bin/sh
# Prepares the volumes, then starts the app. The app itself refuses to start without a
# Calibre library, so a first start on an empty volume gets a new, empty one here.
set -e

for dir in /data "$CALIBRE_LIBRARY"; do
  mkdir -p "$dir" 2>/dev/null || true
  if ! [ -w "$dir" ]; then
    echo "error: $dir is not writable by user $(id -u). Give it to that user, or mount a volume there." >&2
    exit 1
  fi
done
mkdir -p /data/mail-outbox "$CALIBRE_CONFIG_DIRECTORY"

if [ ! -f "$CALIBRE_LIBRARY/metadata.db" ]; then
  echo "no Calibre library in $CALIBRE_LIBRARY; creating an empty one"
  calibredb list --with-library "$CALIBRE_LIBRARY" > /dev/null
fi

exec /app/calibre-ui "$@"
