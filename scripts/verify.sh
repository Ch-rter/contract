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
TOKEN="${TOKEN:-${USDC_ADDRESS:-}}"
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
    if [ -z "${TOKEN}" ]; then
        echo "TOKEN address not set. Run scripts/deploy.sh first or export TOKEN=<contract id>." >&2
        exit 1
    fi
    echo "=== Deploying a treasury ==="
    APPROVER_ADDRESS="$(stellar keys public-key "${APPROVER}")"
    APPROVER2_ADDRESS="$(stellar keys public-key "${APPROVER2}")"
    APPROVERS_JSON="$(mktemp)"
    # shellcheck disable=SC2086
    printf '["%s","%s"]' "${APPROVER_ADDRESS}" "${APPROVER2_ADDRESS}" > "${APPROVERS_JSON}"
    trap 'rm -f "${APPROVERS_JSON}"' EXIT

    # NOTE: --admin must be the identity NAME, not the public key. The CLI only
    # collects auth-entry signers for top-level Address args it can resolve to a
    # stored secret key; a raw G... strkey has no key and fails with
    # "Missing signing key for account G...".
    ORG_ID="$(stellar contract invoke \
        --id "${FACTORY_ADDRESS}" \
        --source-account "${DEPLOYER}" \
        --network "${NETWORK}" \
        -- \
        deploy_treasury \
        --name "Charter Test Org" \
        --admin "${ADMIN}" \
        --approvers-file-path "${APPROVERS_JSON}" \
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
