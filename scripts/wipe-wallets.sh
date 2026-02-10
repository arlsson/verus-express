#!/usr/bin/env bash
#
# Wipes all Lite Wallet data so you can start fresh (onboarding again).
# Run from project root. Quit the app before running.
# macOS only for the app data path.

set -e
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
APP_SUPPORT="$HOME/Library/Application Support/com.maxtheyse.lite-wallet"

echo "Lite Wallet wipe — removing local wallet data..."
echo ""

# Wallet metadata (both possible locations in dev)
for dir in "$PROJECT_ROOT/wallet_data" "$PROJECT_ROOT/src-tauri/wallet_data"; do
  if [ -d "$dir" ]; then
    echo "  Removing $dir"
    rm -rf "$dir"
  fi
done

# macOS app data (Stronghold + any other Tauri app data)
if [ -d "$APP_SUPPORT" ]; then
  echo "  Removing $APP_SUPPORT"
  rm -rf "$APP_SUPPORT"
fi

echo ""
echo "Done. Start the app again to see onboarding."
