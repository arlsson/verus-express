#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ENV_FILE="${LITE_WALLET_ZCASH_PARAMS_ENV_FILE:-$ROOT_DIR/.env.zcash-params}"

SPEND_FILE="sapling-spend.params"
OUTPUT_FILE="sapling-output.params"
SPEND_MIN_BYTES=40000000
OUTPUT_MIN_BYTES=3000000

load_env_file() {
  local file="$1"
  if [[ -f "$file" ]]; then
    # shellcheck disable=SC1090
    source "$file"
  fi
}

require_cmd() {
  local name="$1"
  if ! command -v "$name" >/dev/null 2>&1; then
    echo "[zcash-params] Missing required command: $name" >&2
    exit 1
  fi
}

sha256_of_file() {
  local path="$1"
  shasum -a 256 "$path" | awk '{print tolower($1)}'
}

to_lower() {
  local value="$1"
  printf '%s' "$value" | tr '[:upper:]' '[:lower:]'
}

check_placeholder_and_size() {
  local path="$1"
  local min_bytes="$2"

  if [[ ! -f "$path" ]]; then
    echo "[zcash-params] Missing file: $path" >&2
    exit 1
  fi

  local size
  size="$(wc -c <"$path" | tr -d '[:space:]')"
  if [[ -z "$size" || "$size" -lt "$min_bytes" ]]; then
    echo "[zcash-params] File too small: $path ($size bytes, expected >= $min_bytes)" >&2
    exit 1
  fi

  if LC_ALL=C head -c 8192 "$path" | tr '[:upper:]' '[:lower:]' | grep -q "placeholder"; then
    echo "[zcash-params] Placeholder marker detected in $path" >&2
    exit 1
  fi
}

require_cmd shasum

load_env_file "$ENV_FILE"

PARAMS_DIR="${LITE_WALLET_ZCASH_PARAMS_DIR:-$ROOT_DIR/src-tauri/resources/zcash-params}"
SPEND_PATH="$PARAMS_DIR/$SPEND_FILE"
OUTPUT_PATH="$PARAMS_DIR/$OUTPUT_FILE"

check_placeholder_and_size "$SPEND_PATH" "$SPEND_MIN_BYTES"
check_placeholder_and_size "$OUTPUT_PATH" "$OUTPUT_MIN_BYTES"

if [[ -z "${LITE_WALLET_SAPLING_SPEND_PARAMS_SHA256:-}" ]]; then
  echo "[zcash-params] Missing LITE_WALLET_SAPLING_SPEND_PARAMS_SHA256. Run scripts/bootstrap-zcash-params.sh." >&2
  exit 1
fi

if [[ -z "${LITE_WALLET_SAPLING_OUTPUT_PARAMS_SHA256:-}" ]]; then
  echo "[zcash-params] Missing LITE_WALLET_SAPLING_OUTPUT_PARAMS_SHA256. Run scripts/bootstrap-zcash-params.sh." >&2
  exit 1
fi

SPEND_SHA="$(sha256_of_file "$SPEND_PATH")"
OUTPUT_SHA="$(sha256_of_file "$OUTPUT_PATH")"
EXPECTED_SPEND_SHA="$(to_lower "${LITE_WALLET_SAPLING_SPEND_PARAMS_SHA256}")"
EXPECTED_OUTPUT_SHA="$(to_lower "${LITE_WALLET_SAPLING_OUTPUT_PARAMS_SHA256}")"

if [[ "${SPEND_SHA}" != "${EXPECTED_SPEND_SHA}" ]]; then
  echo "[zcash-params] Spend checksum mismatch." >&2
  echo "expected=$EXPECTED_SPEND_SHA" >&2
  echo "actual=$SPEND_SHA" >&2
  exit 1
fi

if [[ "${OUTPUT_SHA}" != "${EXPECTED_OUTPUT_SHA}" ]]; then
  echo "[zcash-params] Output checksum mismatch." >&2
  echo "expected=$EXPECTED_OUTPUT_SHA" >&2
  echo "actual=$OUTPUT_SHA" >&2
  exit 1
fi

echo "[zcash-params] OK: params present, sane, and checksum-verified."
