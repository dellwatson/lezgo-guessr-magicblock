#!/usr/bin/env bash
set -euo pipefail

# This script creates a new SPL token mint and a treasury token account on devnet.
# It writes the mint and treasury token account pubkeys to:
#   - @programs/reports/01_reward_mint.log
#   - stdout (for easy copy-paste into .env)
#
# It does NOT set mint authority to the program; use 01_set_reward_mint_authority.ts after this.

cd "$(dirname "$0")/.."

# Load env (so we can inherit SOLANA_RPC_URL and SOLANA_PAYER_KEYPAIR)
if [[ -f .env ]]; then
  # shellcheck disable=SC1091
  source .env
fi

# Fallbacks
SOLANA_RPC_URL="${SOLANA_RPC_URL:-https://api.devnet.solana.com}"
export SOLANA_RPC_URL

# Ensure payer keypair is readable
if [[ ! -f "${SOLANA_PAYER_KEYPAIR:?}" ]]; then
  echo "ERROR: SOLANA_PAYER_KEYPAIR file not found: ${SOLANA_PAYER_KEYPAIR}" >&2
  exit 1
fi

# Optional: decimals (default 6)
DECIMALS="${DECIMALS:-6}"
# Optional: token symbol (for logging)
TOKEN_SYMBOL="${TOKEN_SYMBOL:-TESTGO}"

echo "Creating a new SPL token mint on ${SOLANA_RPC_URL}"
echo "Payer keypair: ${SOLANA_PAYER_KEYPAIR}"
echo "Decimals: ${DECIMALS}"
echo "Token symbol (log only): ${TOKEN_SYMBOL}"
echo "---"

# 1) Create the mint
echo "Running: spl-token create-token --decimals ${DECIMALS}"
MINT_OUTPUT=$(spl-token create-token --decimals "${DECIMALS}")
MINT_PUBKEY=$(echo "${MINT_OUTPUT}" | awk '/Creating token/{print $3}')
if [[ -z "${MINT_PUBKEY}" ]]; then
  echo "ERROR: Failed to parse mint pubkey from spl-token output:" >&2
  echo "${MINT_OUTPUT}" >&2
  exit 1
fi
echo "Created mint: ${MINT_PUBKEY}"

# 2) Create a treasury token account owned by the payer
echo "Running: spl-token create-account ${MINT_PUBKEY}"
ACCOUNT_OUTPUT=$(spl-token create-account "${MINT_PUBKEY}")
ACCOUNT_PUBKEY=$(echo "${ACCOUNT_OUTPUT}" | awk '/Creating account/{print $3}')
if [[ -z "${ACCOUNT_PUBKEY}" ]]; then
  echo "ERROR: Failed to parse token account pubkey from spl-token output:" >&2
  echo "${ACCOUNT_OUTPUT}" >&2
  exit 1
fi
echo "Created treasury token account: ${ACCOUNT_PUBKEY}"

# 3) Write a report
REPORT_FILE="reports/01_reward_mint.log"
mkdir -p reports
cat <<EOF >> "${REPORT_FILE}"
timestamp=$(date -u +%Y-%m-%dT%H:%M:%SZ)
TOKEN_SYMBOL=${TOKEN_SYMBOL}
REWARD_MINT=${MINT_PUBKEY}
REWARD_TREASURY_TOKEN_ACCOUNT=${ACCOUNT_PUBKEY}
EOF

echo "Report appended to: ${REPORT_FILE}"
echo "---"
echo "Add these to your .env (or copy into scripts/.env):"
echo "REWARD_MINT=${MINT_PUBKEY}"
echo "REWARD_TREASURY_TOKEN_ACCOUNT=${ACCOUNT_PUBKEY}"
echo "---"
echo "Next step: run 01_set_reward_mint_authority.ts to set mint authority to the program PDA."