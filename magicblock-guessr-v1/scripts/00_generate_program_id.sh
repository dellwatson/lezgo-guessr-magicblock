#!/usr/bin/env bash
set -euo pipefail

# Generate/regenerate the program keypair for magicblock-guessr-v1
# and sync the program ID in src/lib.rs, Anchor.toml, and scripts/.env.

cd "$(dirname "$0")/.."
PROGRAM_ROOT="$(pwd)"
LIB_RS="$PROGRAM_ROOT/src/lib.rs"
ANCHOR_TOML="$PROGRAM_ROOT/../Anchor.toml"
KEYS_DIR="$PROGRAM_ROOT/keys"
ARTIFACT_DIR="$PROGRAM_ROOT/artifacts"

mkdir -p "$KEYS_DIR" "$ARTIFACT_DIR"

PROGRAM_KEYPAIR_PATH_DEFAULT="$KEYS_DIR/guessr_multiplayer_program_v1-keypair.json"
PROGRAM_KEYPAIR_PATH="${PROGRAM_KEYPAIR_PATH:-$PROGRAM_KEYPAIR_PATH_DEFAULT}"

if [[ -f "$PROGRAM_KEYPAIR_PATH" ]]; then
  echo "Found existing program keypair at: $PROGRAM_KEYPAIR_PATH"
  echo "Regenerating (overwriting) program keypair..."
else
  echo "Generating new program keypair at: $PROGRAM_KEYPAIR_PATH"
fi

solana-keygen new --no-bip39-passphrase --force -o "$PROGRAM_KEYPAIR_PATH" >/dev/null
PROGRAM_ID="$(solana-keygen pubkey "$PROGRAM_KEYPAIR_PATH")"

echo "New program ID: $PROGRAM_ID"

echo "Updating src/lib.rs declare_id!()..."
if [[ ! -f "$LIB_RS" ]]; then
  echo "ERROR: lib.rs not found at $LIB_RS" >&2
  exit 1
fi
perl -pi -e 's#^declare_id!\(".*"\);#declare_id!("'$PROGRAM_ID'");#' "$LIB_RS"

echo "Updating Anchor.toml [programs.localnet].guessr_multiplayer_program_v1..."
if [[ ! -f "$ANCHOR_TOML" ]]; then
  echo "WARNING: Anchor.toml not found at $ANCHOR_TOML (skipping)" >&2
else
  perl -pi -e 's#(guessr_multiplayer_program_v1\s*=\s*")([^"]+)(")#$1'$PROGRAM_ID'$3#' "$ANCHOR_TOML"
fi

ENV_FILE="$PROGRAM_ROOT/scripts/.env"
echo "Updating $ENV_FILE with GUESSR_PROGRAM_ID / MULTIPLAYER_PROGRAM_ID / MULTIPLAYER_PROGRAM_KEYPAIR..."
mkdir -p "$(dirname "$ENV_FILE")"
TMP_ENV_FILE="$ENV_FILE.tmp"

if [[ -f "$ENV_FILE" ]]; then
  grep -vE '^(GUESSR_PROGRAM_ID|MULTIPLAYER_PROGRAM_ID|MULTIPLAYER_PROGRAM_KEYPAIR)=' "$ENV_FILE" > "$TMP_ENV_FILE" || true
else
  : > "$TMP_ENV_FILE"
fi

{
  echo "GUESSR_PROGRAM_ID=$PROGRAM_ID"
  echo "MULTIPLAYER_PROGRAM_ID=$PROGRAM_ID"
  echo "MULTIPLAYER_PROGRAM_KEYPAIR=$PROGRAM_KEYPAIR_PATH"
} >> "$TMP_ENV_FILE"

mv "$TMP_ENV_FILE" "$ENV_FILE"

echo ""
echo "✅ Program keypair path : $PROGRAM_KEYPAIR_PATH"
awidTruncated=0
if [[ ${#PROGRAM_ID} -gt 0 ]]; then
  echo "✅ Program ID           : $PROGRAM_ID"
fi

echo ""
echo "Next steps:"
echo "  1) Deploy with:   ./scripts/02_deploy_multiplayer_program.sh"
echo "  2) Ensure your runtime env loads scripts/.env so GUESSR_PROGRAM_ID is set."
