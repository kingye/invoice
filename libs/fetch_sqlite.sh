#!/bin/bash
set -e
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
SQLITE_DIR="$SCRIPT_DIR"
SQLITE_VERSION="3450300"
SQLITE_URL="https://www.sqlite.org/2024/sqlite-amalgamation-${SQLITE_VERSION}.zip"
ZIP_FILE="$SQLITE_DIR/sqlite-amalgamation.zip"

if [ -f "$SQLITE_DIR/sqlite3.c" ] && [ -f "$SQLITE_DIR/sqlite3.h" ]; then
    exit 0
fi

echo "Fetching SQLite amalgamation..."
curl -sL "$SQLITE_URL" -o "$ZIP_FILE"
unzip -o "$ZIP_FILE" -d "$SQLITE_DIR" > /dev/null
cp "$SQLITE_DIR/sqlite-amalgamation-${SQLITE_VERSION}/sqlite3.c" "$SQLITE_DIR/"
cp "$SQLITE_DIR/sqlite-amalgamation-${SQLITE_VERSION}/sqlite3.h" "$SQLITE_DIR/"
rm -rf "$SQLITE_DIR/sqlite-amalgamation-${SQLITE_VERSION}" "$ZIP_FILE"
echo "SQLite amalgamation ready."
