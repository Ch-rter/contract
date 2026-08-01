#!/usr/bin/env bash
# Verifies the charter factory deployment by exercising its read paths.
#
# Loads the factory address and treasury wasm hash from scripts/.env (written
# by deploy.sh) and confirms the factory is initialized, reports an org count,
# and can deploy + return an org record.
#
# Usage: ./scripts/verify.sh [deploy-treasury]
set -euo pipefail

cd "$(dirname "$0")/.."

NETWORK="${NETWORK:-testnet}"
DEPLOYER="${DEPLOYER:-charter-deployer}"
ADMIN="${ADMIN:-charter-admin}"
APPROVER="${APPROVER:-charter-approver}"
APPROVER2="${APPROVER2:-charter-approver2}"
TOKEN="${TOKEN:-USDC}"   # issuer-neutral symbol; override with a token address as needed
ENV_FILE="scripts/.env"

[ -f "${ENV_FILE}" ] && source "${ENV_FILE}"

if [ -z "${FACTORY_ADDRESS:-}" ]; then
    echo "FACTORY_ADDRESS not set. Run scripts/deploy.sh first." >&2
    exit 1
fi

echo "=== Factory read checks ==="
echo "Factory address: ${FACTORY_ADDRESS}"
echo -n "Org count: "
stellar contract invoke \
    --id "${FACTORY_ADDRESS}" \
    --source-account "${DEPLOYER}" \
    --network "${NETWORK}" \
    -- \
    get_org_count
echo -n "Org page (start=1, limit=5): "
stellar contract invoke \
    --id "${FACTORY_ADDRESS}" \
    --source-account "${DEPLOYER}" \
    --network "${NETWORK}" \
    -- \
    get_orgs \
    --start 1 \
    --limit 5

if [ "${1:-}" = "deploy-treasury" ]; then
    echo "=== Deploying a treasury ==="
    ADMIN_ADDRESS="$(stellar keys public-key "${ADMIN}")"
    APPROVER_ADDRESS="$(stellar keys public-key "${APPROVER}")"
    APPROVER2_ADDRESS="$(stellar keys public-key "${APPROVER2}")"

    ORG_ID="$(stellar contract invoke \
        --id "${FACTORY_ADDRESS}" \
        --source-account "${DEPLOYER}" \
        --network "${NETWORK}" \
        -- \
        deploy_treasury \
        --name "Charter Test Org" \
        --admin "${ADMIN_ADDRESS}" \
        --approvers "[\"${APPROVER_ADDRESS}\",\"${APPROVER2_ADDRESS}\"]" \
        --threshold 2 \
        --token "${TOKEN}" 2>&1 | tail -n 1)"
    echo "Deployed org id: ${ORG_ID}"

    echo "=== Org record ==="
    stellar contract invoke \
        --id "${FACTORY_ADDRESS}" \
        --source-account "${DEPLOYER}" \
        --network "${NETWORK}" \
        -- \
        get_org \
        --org_id "${ORG_ID}"
fi

echo
echo "Verification complete."
