# Security Policy

## Supported versions

Charter is pre-1.0 and under active development. Security fixes are applied to
the `main` branch and the latest tagged release. Older tags are not maintained.

| Version | Supported |
|---------|-----------|
| `main` / latest release | ✅ |
| Older tags | ❌ |

## Reporting a vulnerability

**Please do not report security vulnerabilities through public GitHub issues,
pull requests, or discussions.**

Report vulnerabilities privately through either channel:

1. **GitHub Private Vulnerability Reporting (preferred).** Open a report at
   [github.com/Ch-rter/contract/security/advisories/new](https://github.com/Ch-rter/contract/security/advisories/new).
   This keeps the report private to the maintainers until a fix is published.
2. **Email.** Send details to **fuhadadesanya0@gmail.com** with a subject line
   beginning `[SECURITY]`.

Please include as much of the following as you can:

- The affected contract (treasury, factory, or test-token) and function.
- A description of the vulnerability and its impact (e.g. funds at risk,
  bypassed approval threshold, unauthorized deployment).
- Steps to reproduce, ideally a failing test or a testnet transaction hash.
- Any suggested remediation.

## What to expect

- **Acknowledgement** within 5 business days.
- An initial assessment and severity rating after triage.
- Coordinated disclosure: we will agree on a disclosure timeline with you and
  credit you in the advisory unless you prefer to remain anonymous.

## Scope

In scope:

- The Soroban contracts under `contracts/` (`treasury`, `factory`).
- Deployment and verification scripts under `scripts/` where a flaw would lead
  to an insecure on-chain deployment.

Out of scope:

- `contracts/test-token`, which is a minimal token used only for tests and
  on-chain verification and is not intended for production use.
- Issues that require a compromised deployer key, admin key, or approver key.
  Charter's security model assumes these signers are honest and their keys are
  kept secret; loss or compromise of a threshold of keys is outside the trust
  model.
- Denial of service from unbounded on-chain reads (a known limitation tracked
  in the issue tracker), unless it enables loss of funds.

Thank you for helping keep Charter and its users safe.
