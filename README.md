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

> The pinned toolchain in `rust-toolchain.toml` targets a Windows (`windows-gnu`) build host. The contracts are platform-independent Soroban wasm; only the local build toolchain is currently pinned to Windows.

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

