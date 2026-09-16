#!/usr/bin/env bash

# qutemarks userscript for qutebrowser
# Adds the current page to the local bookmark manager.

if [ -z "$QUTE_URL" ]; then
    echo "message-error 'Not running inside qutebrowser!'" >> "$QUTE_FIFO"
    exit 1
fi

PAYLOAD=$(cat <<JSON
{
  "url": "$QUTE_URL",
  "title": "$QUTE_TITLE"
}
JSON
)

RESPONSE=$(curl -s -o /dev/null -w "%{http_code}" -X POST -H "Content-Type: application/json" -d "$PAYLOAD" http://127.0.0.1:8338/api/bookmarks)

if [ "$RESPONSE" -eq 201 ]; then
    echo "message-info 'Bookmark saved: $QUTE_TITLE'" >> "$QUTE_FIFO"
else
    echo "message-error 'Failed to save bookmark (HTTP $RESPONSE)'" >> "$QUTE_FIFO"
fi
