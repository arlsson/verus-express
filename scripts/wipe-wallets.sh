#!/usr/bin/env bash
#
# Wipes all Lite Wallet data so you can start fresh (onboarding again).
# Run from project root. Quit the app before running.
# macOS only for the app data path.

set -e
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
TAURI_CONF="$PROJECT_ROOT/src-tauri/tauri.conf.json"
LEGACY_APP_ID="com.maxtheyse.lite-wallet"
LEGACY_WEBKIT_PROFILE="lite-wallet"

to_webkit_profile_name() {
  printf '%s' "$1" \
    | tr '[:upper:]' '[:lower:]' \
    | sed -e 's/[^a-z0-9]/_/g' -e 's/__*/_/g' -e 's/^_//' -e 's/_$//'
}

# Read current Tauri bundle identifier from tauri.conf.json (macOS app data path key)
APP_ID_FROM_CONFIG=""
PRODUCT_NAME_FROM_CONFIG=""
if [ -f "$TAURI_CONF" ]; then
  APP_ID_FROM_CONFIG="$(
    sed -n 's/^[[:space:]]*"identifier":[[:space:]]*"\([^"]*\)".*/\1/p' "$TAURI_CONF" | head -n 1
  )"
  PRODUCT_NAME_FROM_CONFIG="$(
    sed -n 's/^[[:space:]]*"productName":[[:space:]]*"\([^"]*\)".*/\1/p' "$TAURI_CONF" | head -n 1
  )"
fi

echo "Lite Wallet wipe — removing local wallet data..."
echo ""

# Wallet metadata (both possible locations in dev)
for dir in "$PROJECT_ROOT/wallet_data" "$PROJECT_ROOT/src-tauri/wallet_data"; do
  if [ -d "$dir" ]; then
    echo "  Removing $dir"
    rm -rf "$dir"
  fi
done

# macOS app data (Stronghold + wallet metadata + any Tauri app state)
APP_DATA_DIRS=()
if [ -n "$APP_ID_FROM_CONFIG" ]; then
  APP_DATA_DIRS+=("$HOME/Library/Application Support/$APP_ID_FROM_CONFIG")
fi
APP_DATA_DIRS+=("$HOME/Library/Application Support/$LEGACY_APP_ID")

for app_dir in "${APP_DATA_DIRS[@]}"; do
  if [ -d "$app_dir" ]; then
    echo "  Removing $app_dir"
    rm -rf "$app_dir"
  fi
done

# WebKit website data (includes frontend localStorage, e.g. locale + language gate flag)
WEBKIT_PROFILE_FROM_PRODUCT=""
WEBKIT_PROFILE_FROM_APP_ID=""
if [ -n "$PRODUCT_NAME_FROM_CONFIG" ]; then
  WEBKIT_PROFILE_FROM_PRODUCT="$(to_webkit_profile_name "$PRODUCT_NAME_FROM_CONFIG")"
fi
if [ -n "$APP_ID_FROM_CONFIG" ]; then
  WEBKIT_PROFILE_FROM_APP_ID="$(to_webkit_profile_name "${APP_ID_FROM_CONFIG##*.}")"
fi

WEBKIT_DIRS=()
if [ -n "$WEBKIT_PROFILE_FROM_PRODUCT" ]; then
  WEBKIT_DIRS+=("$HOME/Library/WebKit/$WEBKIT_PROFILE_FROM_PRODUCT")
fi
if [ -n "$WEBKIT_PROFILE_FROM_APP_ID" ]; then
  WEBKIT_DIRS+=("$HOME/Library/WebKit/$WEBKIT_PROFILE_FROM_APP_ID")
fi
WEBKIT_DIRS+=("$HOME/Library/WebKit/$LEGACY_WEBKIT_PROFILE")

for webkit_dir in "${WEBKIT_DIRS[@]}"; do
  if [ -d "$webkit_dir" ]; then
    echo "  Removing $webkit_dir"
    rm -rf "$webkit_dir"
  fi
done

echo ""
echo "Done. Start the app again to see onboarding."
