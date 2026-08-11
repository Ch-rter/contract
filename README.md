# Charter

[![CI](https://github.com/Ch-rter/contract/actions/workflows/ci.yml/badge.svg)](https://github.com/Ch-rter/contract/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Stellar](https://img.shields.io/badge/Stellar-Testnet-brightgreen.svg)](https://stellar.org)
[![Built with Soroban](https://img.shields.io/badge/Built%20with-Soroban-blue.svg)](https://soroban.stellar.org)

Charter is a set of Soroban smart contracts for on-chain treasury management. An organization deploys a treasury with a set of approvers and an approval threshold, organizes its funds into budget categories with lifetime spending caps, and releases funds only when enough approvers sign off. A factory contract deploys and tracks these treasuries so many organizations can run independent treasuries from a single, verifiable deployment. Every balance, category, request, and approval is stored on-chain and publicly readable.

## Maintainers

| Name | Role | GitHub |
|------|------|--------|
| Fuhad | Lead maintainer | [@fadesany](https://github.com/fadesany) |

## What is this?

Charter splits treasury management across two contracts:

- A **treasury** holds a single token for one organization. Funds are grouped into budget categories, each with a lifetime cap. Spending happens through a request-and-approval flow: a member submits a request against a category, approvers sign it, and once the approval threshold is met the payout executes automatically.
- A **factory** deploys and registers treasuries. Each treasury is deployed from a single verified wasm hash with a deterministic address, so every organization runs the same reviewed code.

This design suits DAOs, grant programs, and any group that needs multi-party control over shared funds with an auditable, on-chain record.

## Deployed contracts (Testnet)

Network: **Testnet** (`Test SDF Network ; September 2015`)

| Contract | Address | Wasm hash |
|----------|---------|-----------|
| Factory | `CCUQBFFRGR4RUWHKLWSRWKBL3WORHNTHFLTKMHTNUZL4T5733ODN5WD4` | `e6ee93a93dd18927abab8dc1c4f95ec820da020310b2b2a45a0588b91581df8a` |
| Treasury (reference deployment) | `CAH4PUADD2X3K52TKETWTIL4GHPZT55LWUEVVOSH6B3D3KA2ZH7HQGTT` | `b72f664802f395192375b4fca2e0930cff6f994a8053ca568d4f96eb0032ba6c` |

Treasuries are normally created through the factory's `deploy_treasury`. The treasury above is one reference deployment kept for verification; both wasm hashes are reproducible from `stellar contract fetch`.

## Quick start

### Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) `1.92.0` (pinned in `rust-toolchain.toml`, which also adds the `wasm32v1-none` target)
- [Stellar CLI](https://developers.stellar.org/docs/tools/cli/install-cli) `26.x` (`stellar 26.1.0` is the tested version)

> CI builds and tests on Linux, and the contracts are platform-independent Soroban wasm. `.cargo/config.toml` carries a Windows (`windows-gnu`) linker override for contributors who build natively on Windows; it is inert on other hosts.

### Build and test

```bash
# Compile the contracts to wasm
stellar contract build

# Run the test suite (47 treasury + 11 factory = 58 tests)
cargo test
```

### Deploy to testnet

The scripts under `scripts/` deploy and exercise the contracts end to end. Run them in order:

```bash
# 1. Create and fund testnet identities (deployer, admin, approvers, requester)
./scripts/setup-testnet.sh

# 2. Build, upload the treasury wasm, deploy the factory, and initialize it.
#    Writes the resulting factory address + treasury wasm hash to scripts/.env
./scripts/deploy.sh

# 3. Verify the deployment by exercising the factory's read paths.
#    Pass `deploy-treasury` to also deploy a treasury through the factory.
./scripts/verify.sh
./scripts/verify.sh deploy-treasury
```

The factory must be initialized with the treasury's wasm hash before it can deploy treasuries — `deploy.sh` handles this ordering (upload treasury wasm → deploy factory → `initialize`).

## Architecture

```
contracts/
├── treasury/      # Per-organization treasury: categories, caps, approval flow
├── factory/       # Deploys and registers treasuries from the treasury wasm hash
└── test-token/    # Minimal mintable token used only in tests and verification
scripts/
├── setup-testnet.sh   # Create + fund testnet identities
├── deploy.sh          # Build, upload, deploy, initialize
└── verify.sh          # Exercise factory read/deploy paths on-chain
```

### Treasury

A treasury holds one token for one organization and is controlled by a set of approvers with an approval threshold. Its lifecycle:

1. **Initialize** with an admin, approvers, threshold, and token.
2. **Create categories**, each with a lifetime spending cap.
3. **Deposit** the token into the treasury.
4. **Submit a request** to spend from a category.
5. **Approve** — when approvals reach the threshold, the payout executes automatically and the category's spent total increases. Requests can also be rejected or cancelled.

Caps are lifetime totals: a category tracks cumulative `spent` against its `cap` and never resets.

### Factory

The factory is initialized once with the treasury wasm hash. Each `deploy_treasury` call deploys a treasury at a deterministic address (salted by a sequential org id), initializes it, and records an on-chain org registry entry. Reads are available through `get_org`, `get_org_count`, and the paginated `get_orgs`.

## Contract reference

Every entry point below is a public contract function. The `env: Env` host parameter is injected by the runtime, so signatures are shown as a caller sees them — e.g. `client.initialize(&admin, &approvers, &threshold, &token)` from a generated client, or `stellar contract invoke … -- initialize --admin … --approvers … --threshold … --token …` from the CLI. All `i128` amounts are in the token's smallest unit (scaled by its `decimals`).

### Treasury

Per-organization vault. Configuration (admin, approvers, threshold, token) lives in instance storage; categories and requests live in persistent storage.

**Configuration & approvers** — admin only:

```rust
fn initialize(admin: Address, approvers: Vec<Address>, threshold: u32, token: Address)
fn add_approver(admin: Address, approver: Address)     // no-op if already an approver
fn remove_approver(admin: Address, approver: Address)  // fails if it would drop below threshold
fn set_threshold(admin: Address, threshold: u32)
```

**Budget categories** — admin only:

```rust
fn create_category(admin: Address, name: String, cap: i128) -> u32       // cap > 0; returns category_id
fn update_category_cap(admin: Address, category_id: u32, new_cap: i128)   // new_cap >= spent
fn set_category_active(admin: Address, category_id: u32, active: bool)
```

**Funds & requests:**

```rust
fn deposit(from: Address, amount: i128)                                   // auth: from
fn submit_request(requester: Address, category_id: u32, recipient: Address, amount: i128, memo: String) -> u32  // auth: requester; returns request_id
fn approve_request(approver: Address, request_id: u32)                    // auth: approver; auto-executes at threshold
fn reject_request(approver: Address, request_id: u32)                     // auth: approver
fn cancel_request(requester: Address, request_id: u32)                    // auth: original requester
```

**Views** (no auth):

```rust
fn get_category(category_id: u32) -> Category
fn get_categories() -> Vec<Category>
fn get_request(request_id: u32) -> Request
fn get_requests_by_category(category_id: u32) -> Vec<Request>
fn get_balance() -> i128
fn get_approvers() -> Vec<Address>
fn get_threshold() -> u32
```

> `get_categories` and `get_requests_by_category` iterate every entity with no upper bound; on treasuries with many categories or requests they can exceed transaction resource limits. Pagination is tracked in [issue #3](https://github.com/Ch-rter/contract/issues/3).

**Data types:**

```rust
struct Category { name: String, cap: i128, spent: i128, active: bool }

enum RequestStatus { Pending, Executed, Rejected, Cancelled }

struct Request {
    id: u32,
    category_id: u32,
    recipient: Address,
    amount: i128,
    memo: String,
    requester: Address,
    approvals: Vec<Address>,
    status: RequestStatus,
    created_ledger: u32,
}
```

**Events:**

| Event | Topics | Data |
|-------|--------|------|
| `CategoryCreated` | `category_id` | `name`, `cap` |
| `CapUpdated` | `category_id` | `new_cap` |
| `ActiveChanged` | `category_id` | `active` |
| `Deposited` | `from` | `amount` |
| `RequestSubmitted` | `request_id` | `category_id`, `recipient`, `amount` |
| `RequestApproved` | `request_id` | `approver` |
| `RequestExecuted` | `request_id` | `recipient`, `amount` |
| `RequestRejected` | `request_id` | `approver` |
| `RequestCancelled` | `request_id` | — |

**Errors:**

| Code | Name | Raised when |
|------|------|-------------|
| 1 | `AlreadyInitialized` | `initialize` is called a second time |
| 2 | `NotInitialized` | a function is called before `initialize` |
| 3 | `NotAdmin` | a non-admin calls an admin-only function |
| 4 | `NotApprover` | approve/reject is called by an address outside the approver set |
| 5 | `CategoryInactive` | a request is submitted against an inactive category |
| 6 | `CapExceeded` | reserved — cap overruns currently surface as `InvalidAmount` |
| 7 | `RequestNotPending` | the target request is not pending (or does not exist) |
| 8 | `InvalidThreshold` | threshold is 0, exceeds the approver count, or a removal would drop below it |
| 9 | `AlreadyApproved` | the same approver approves a request twice |
| 10 | `NotRequester` | a non-submitter tries to cancel a request |
| 11 | `InvalidAmount` | non-positive amount, insufficient remaining cap, or unknown category |

### Factory

Deploys treasuries from a single uploaded treasury wasm and records each as an org. Deploy authority is a single `deployer` account.

```rust
fn initialize(deployer: Address, wasm_hash: BytesN<32>)                   // auth: deployer
fn deploy_treasury(name: String, admin: Address, approvers: Vec<Address>, threshold: u32, token: Address) -> u32  // auth: deployer + admin; returns org_id
fn get_org(org_id: u32) -> OrgRecord
fn get_org_count() -> u32
fn get_orgs(start: u32, limit: u32) -> Vec<OrgRecord>                     // limit capped at 50
```

`deploy_treasury` requires **both** the `deployer` and the new treasury's `admin` to sign: the admin signature is needed because the factory immediately sub-calls the treasury's `initialize`, which itself requires `admin` auth. The org id doubles as the deploy salt, so every treasury address is deterministic.

**Data types:**

```rust
struct OrgRecord { name: String, treasury: Address, admin: Address, created_ledger: u32 }
```

**Events:**

| Event | Topics | Data |
|-------|--------|------|
| `TreasuryDeployed` | `org_id` | `name`, `treasury`, `admin` |

> `initialize` does not currently emit an event; adding `FactoryInitialized` is tracked in [issue #2](https://github.com/Ch-rter/contract/issues/2).

**Errors:**

| Code | Name | Raised when |
|------|------|-------------|
| 1 | `NotInitialized` | `deploy_treasury` is called before `initialize` |
| 2 | `AlreadyInitialized` | `initialize` is called a second time |
| 3 | `NotDeployer` | reserved — deploy authority is enforced via `require_auth` on the stored deployer |
| 4 | `OrgNotFound` | `get_org` is called with an unknown id |

### Test token

`contracts/test-token` is a minimal mintable token used only by the test suite and the on-chain verification scripts — it is **not** part of the production surface (see [SECURITY.md](SECURITY.md)). It implements just enough of a token interface for a treasury to hold and move balances:

```rust
fn init(admin: Address, decimal: u32, name: String, symbol: String)
fn mint(to: Address, amount: i128)                       // auth: admin
fn transfer(from: Address, to: Address, amount: i128)    // auth: from
fn balance(id: Address) -> i128
fn decimals() -> u32
fn name() -> String
fn symbol() -> String
fn admin() -> Address
```

## Contributing

Contributions are welcome. To get started:

1. Browse the [open issues](https://github.com/Ch-rter/contract/issues) — issues labelled `good first issue` are a good entry point.
2. Fork the repo and create a branch (`feat/…`, `fix/…`, or `docs/…`).
3. Make your change and ensure `cargo test` passes and `stellar contract build` succeeds.
4. Open a pull request against `main` with a clear description. Commits follow [Conventional Commits](https://www.conventionalcommits.org/) (`feat(scope):`, `fix(scope):`, `docs(scope):`).

See [SECURITY.md](SECURITY.md) for how to report vulnerabilities.

## License

Licensed under the [MIT License](LICENSE).

## Contributors

[![Contributors](https://contrib.rocks/image?repo=Ch-rter/contract)](https://github.com/Ch-rter/contract/graphs/contributors)

