#!/usr/bin/env bash
# Deploys the charter factory contract to a Stellar network.
#
# The treasury wasm is uploaded first; its hash is passed to the factory's
# constructor, so it is set in the same transaction that creates the factory
# and no one can set a different hash first. The resulting
# factory address and treasury wasm hash are written to scripts/.env so that
# verify.sh can use them.
#
# Prerequisites: run scripts/setup-testnet.sh first.
#
# Usage: ./scripts/deploy.sh
set -euo pipefail

cd "$(dirname "$0")/.."

NETWORK="${NETWORK:-testnet}"
DEPLOYER="${DEPLOYER:-charter-deployer}"
ENV_FILE="scripts/.env"

# shellcheck source=scripts/.env
[ -f "${ENV_FILE}" ] && source "${ENV_FILE}"

echo "=== Building contracts ==="
stellar contract build

TREASURY_WASM="target/wasm32v1-none/release/charter_treasury.wasm"
FACTORY_WASM="target/wasm32v1-none/release/charter_factory.wasm"

echo "=== Uploading treasury wasm ==="
TREASURY_HASH="$(stellar contract upload \
    --wasm "${TREASURY_WASM}" \
    --source-account "${DEPLOYER}" \
    --network "${NETWORK}" \
    2>&1 | tail -n 1)"
echo "Treasury wasm hash: ${TREASURY_HASH}"

echo "=== Deploying factory ==="
FACTORY_ADDRESS="$(stellar contract deploy \
    --wasm "${FACTORY_WASM}" \
    --source-account "${DEPLOYER}" \
    --network "${NETWORK}" \
    -- \
    --wasm_hash "${TREASURY_HASH}" \
    2>&1 | tail -n 1)"
echo "Factory address: ${FACTORY_ADDRESS}"
# The arguments after `--` go to the factory's constructor, which runs inside
# the creating transaction. There is no separate initialize step. DEPLOYER only
# pays the fees; it has no role in the contract.

cat > "${ENV_FILE}" <<EOF
NETWORK="${NETWORK}"
DEPLOYER="${DEPLOYER}"
TREASURY_WASM_HASH="${TREASURY_HASH}"
FACTORY_ADDRESS="${FACTORY_ADDRESS}"
EOF
chmod 600 "${ENV_FILE}"

echo
echo "Deployment complete."
echo "  Treasury wasm hash: ${TREASURY_HASH}"
echo "  Factory address:    ${FACTORY_ADDRESS}"
echo "  Saved to: ${ENV_FILE}"
