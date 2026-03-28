#!/bin/bash
set -e

# Fix ownership of database file if it exists and is root-owned
DB_FILE="/data/data.db"
if [ -f "$DB_FILE" ] && [ "$(stat -c '%U' "$DB_FILE" 2>/dev/null)" = "root" ]; then
    echo "Fixing ownership of $DB_FILE"
    chown mlm:mlm "$DB_FILE"
fi

# Execute the main process
exec /mlm "$@"