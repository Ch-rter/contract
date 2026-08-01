#!/usr/bin/env bash
# Prepares identities on Stellar testnet for the charter contracts.
#
# Creates three identities if they don't exist and funds them via the testnet
# friendbot: the deployer (owns the factory), the org admin, and an approver.
#
# Usage: ./scripts/setup-testnet.sh
set -euo pipefail

cd "$(dirname "$0")/.."

NETWORK="${NETWORK:-testnet}"
IDENTITIES=("charter-deployer" "charter-admin" "charter-approver" "charter-approver2" "charter-requester")

echo "Using network: ${NETWORK}"

for id in "${IDENTITIES[@]}"; do
    if ! stellar keys ls 2>/dev/null | grep -q "^${id}\$"; then
        echo "Creating identity: ${id}"
        stellar keys generate --network "${NETWORK}" "${id}" --fund 2>/dev/null \
            || stellar keys generate --network "${NETWORK}" "${id}"
        echo "Funding ${id} via friendbot..."
        stellar keys fund "${id}" --network "${NETWORK}" 2>/dev/null || true
    else
        echo "Identity ${id} already exists."
        stellar keys fund "${id}" --network "${NETWORK}" 2>/dev/null || true
    fi
    addr="$(stellar keys public-key "${id}")"
    echo "  ${id}: ${addr}"
done

echo
echo "Setup complete. Identities:"
stellar keys ls
