#!/bin/bash
set -e
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
SQLITE_DIR="$SCRIPT_DIR"
SQLITE_VERSION="3450300"
SQLITE_URL="https://www.sqlite.org/2024/sqlite-amalgamation-${SQLITE_VERSION}.zip"
SQLITE_SHA256="ea170e73e447703e8359308ca2e4366a3ae0c4304a8665896f068c736781c651"
ZIP_FILE="$SQLITE_DIR/sqlite-amalgamation.zip"

if [ -f "$SQLITE_DIR/sqlite3.c" ] && [ -f "$SQLITE_DIR/sqlite3.h" ]; then
    exit 0
fi

echo "Fetching SQLite amalgamation..."
curl -sL "$SQLITE_URL" -o "$ZIP_FILE"

ACTUAL_SHA256=$(sha256sum "$ZIP_FILE" | cut -d' ' -f1)
if [ "$ACTUAL_SHA256" != "$SQLITE_SHA256" ]; then
    echo "SHA-256 mismatch! Expected $SQLITE_SHA256, got $ACTUAL_SHA256" >&2
    rm -f "$ZIP_FILE"
    exit 1
fi

unzip -o "$ZIP_FILE" -d "$SQLITE_DIR" > /dev/null
cp "$SQLITE_DIR/sqlite-amalgamation-${SQLITE_VERSION}/sqlite3.c" "$SQLITE_DIR/"
cp "$SQLITE_DIR/sqlite-amalgamation-${SQLITE_VERSION}/sqlite3.h" "$SQLITE_DIR/"
rm -rf "$SQLITE_DIR/sqlite-amalgamation-${SQLITE_VERSION}" "$ZIP_FILE"
echo "SQLite amalgamation ready (verified)."
