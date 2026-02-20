#!/usr/bin/env bash
#
# Wipes only dLight z-address sync data so private channels must resync.
# Run from project root. Quit the app before running.
# macOS only for the app data path.

set -e
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
TAURI_CONF="$PROJECT_ROOT/src-tauri/tauri.conf.json"
LEGACY_APP_ID="com.maxtheyse.lite-wallet"

# Read current Tauri bundle identifier from tauri.conf.json (macOS app data path key)
APP_ID_FROM_CONFIG=""
if [ -f "$TAURI_CONF" ]; then
  APP_ID_FROM_CONFIG="$(
    sed -n 's/^[[:space:]]*"identifier":[[:space:]]*"\([^"]*\)".*/\1/p' "$TAURI_CONF" | head -n 1
  )"
fi

echo "Lite Wallet wipez - removing dLight sync state only..."
echo ""

APP_DATA_DIRS=()
if [ -n "$APP_ID_FROM_CONFIG" ]; then
  APP_DATA_DIRS+=("$HOME/Library/Application Support/$APP_ID_FROM_CONFIG")
fi
APP_DATA_DIRS+=("$HOME/Library/Application Support/$LEGACY_APP_ID")

REMOVED_ANY=0
for app_dir in "${APP_DATA_DIRS[@]}"; do
  dlight_dir="$app_dir/dlight"
  if [ -d "$dlight_dir" ]; then
    echo "  Removing $dlight_dir"
    rm -rf "$dlight_dir"
    REMOVED_ANY=1
  fi
done

if [ "$REMOVED_ANY" -eq 0 ]; then
  echo "  No dLight directory found."
fi

echo ""
echo "Done. Start the app again and private balances will resync."
