#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SOLANA_RPC_URL="${SOLANA_RPC_URL:-https://api.devnet.solana.com}"
SOLANA_PAYER_KEYPAIR="${SOLANA_PAYER_KEYPAIR:-$HOME/.config/solana/id.json}"
MULTIPLAYER_PROGRAM_KEYPAIR="${MULTIPLAYER_PROGRAM_KEYPAIR:-$ROOT_DIR/keys/multiplayer-program-keypair.json}"
PROGRAM_CRATE_DIR="$ROOT_DIR/magicblock-guessr"
ARTIFACT_DIR="$ROOT_DIR/artifacts"

mkdir -p "$ROOT_DIR/keys" "$ARTIFACT_DIR"

if [[ ! -f "$MULTIPLAYER_PROGRAM_KEYPAIR" ]]; then
  solana-keygen new --no-bip39-passphrase --force -o "$MULTIPLAYER_PROGRAM_KEYPAIR" >/dev/null
fi

echo "[1/2] Building multiplayer program"
cargo build-sbf --manifest-path "$PROGRAM_CRATE_DIR/Cargo.toml" --sbf-out-dir "$ARTIFACT_DIR"

PROGRAM_SO="$ARTIFACT_DIR/guessr_multiplayer_program.so"
if [[ ! -f "$PROGRAM_SO" ]]; then
  echo "Program artifact not found: $PROGRAM_SO"
  exit 1
fi

echo "[2/2] Deploying multiplayer program"
solana program deploy "$PROGRAM_SO" \
  --url "$SOLANA_RPC_URL" \
  --keypair "$SOLANA_PAYER_KEYPAIR" \
  --program-id "$MULTIPLAYER_PROGRAM_KEYPAIR"

MULTIPLAYER_PROGRAM_ID="$(solana-keygen pubkey "$MULTIPLAYER_PROGRAM_KEYPAIR")"
echo "Guessr program id: ${MULTIPLAYER_PROGRAM_ID}"
echo "export GUESSR_PROGRAM_ID=${MULTIPLAYER_PROGRAM_ID}"
