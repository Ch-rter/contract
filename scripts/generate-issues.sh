#!/usr/bin/env bash
# Batch-creates the repo's issue labels and the initial backlog of real,
# code-verified gaps. Idempotent: labels are upserted with --force and issues
# are skipped if an issue with the same title already exists.
#
# Usage: ./scripts/generate-issues.sh
#   REPO=owner/name ./scripts/generate-issues.sh   # override target repo
#
# Requires: gh (authenticated with repo scope).
set -euo pipefail

REPO="${REPO:-Ch-rter/contract}"

echo "=== Labels ==="
# label <name> <color> <description>
label() {
    gh label create "$1" --repo "$REPO" --color "$2" --description "$3" --force
}
label "bug"              "d73a4a" "Something isn't working"
label "enhancement"      "a2eeef" "New feature or request"
label "security"         "b60205" "Security-relevant issue"
label "good first issue" "7057ff" "Good for newcomers"
label "documentation"    "0075ca" "Improvements or additions to documentation"

echo
echo "=== Issues ==="
# Snapshot existing titles once so the script is safe to re-run.
existing_titles="$(gh issue list --repo "$REPO" --state all --limit 200 --json title --jq '.[].title')"

# create_issue <title> <body> [--label X ...]
create_issue() {
    local title="$1" body="$2"
    shift 2
    if grep -Fxq "$title" <<<"$existing_titles"; then
        echo "skip (exists): $title"
        return
    fi
    gh issue create --repo "$REPO" --title "$title" --body "$body" "$@"
}

create_issue \
    "factory: emit an event from initialize" \
    "\`deploy_treasury\` publishes a \`TreasuryDeployed\` event (contracts/factory/src/lib.rs:144-150), but \`initialize\` (contracts/factory/src/lib.rs:48-57) writes the deployer and treasury wasm hash to instance storage without emitting anything.

Off-chain indexers can't observe when a factory was initialized, by whom, or which treasury wasm hash it was bound to without reading instance storage directly.

**Proposed:** emit a \`FactoryInitialized { deployer, wasm_hash }\` event at the end of \`initialize\`, mirroring the event pattern already used elsewhere in the contract." \
    --label "enhancement" --label "good first issue"

create_issue \
    "treasury: paginate get_categories and get_requests_by_category" \
    "\`get_categories\` (contracts/treasury/src/lib.rs:499-512) and \`get_requests_by_category\` (contracts/treasury/src/lib.rs:526-545) both iterate \`1..=count\` over persistent storage with no upper bound. As the number of categories or requests grows, these calls consume unbounded read/instruction budget and will eventually exceed the transaction resource limits, making the data unretrievable through these entrypoints.

The factory already solves exactly this: \`get_orgs\` caps \`limit\` at \`MAX_ORG_LIMIT = 50\` and takes \`(start, limit)\` (contracts/factory/src/lib.rs:25, 176-187).

**Proposed:** apply the same \`(start, limit)\` pagination pattern to both treasury view functions. This is the unbounded-read limitation referenced in SECURITY.md." \
    --label "security" --label "enhancement"

create_issue \
    "treasury: support recurring budget caps that reset per period" \
    "A \`Category\` cap is a lifetime total: \`spent\` accumulates and never resets (contracts/treasury/src/lib.rs:160-165, and documented in the README — \"Caps are lifetime totals … and never resets\").

Organizations that budget per month or quarter can't express \"5,000 per month\" without deploying a fresh category and manually deactivating the old one.

**Proposed:** an optional period on a category (reset interval + last-reset ledger) so recurring budgets reset automatically. The current lifetime-cap behavior is intentional and should remain the default; this is an additive enhancement." \
    --label "enhancement"

create_issue \
    "scripts: verify.sh may read an org before the network catches up" \
    "In the \`deploy-treasury\` path, verify.sh invokes \`deploy_treasury\` and then immediately calls \`get_org --org_id \"\$ORG_ID\"\` (scripts/verify.sh:64-84). These are two separate CLI round-trips to the RPC; the read can race the deploy transaction's propagation and intermittently fail with \`OrgNotFound\` (or return a stale result) on a slow testnet.

**Proposed:** wrap the \`get_org\` read in a short retry/poll loop (a few attempts with a sleep between) so verification is deterministic." \
    --label "bug" --label "good first issue"

echo
echo "Done."
