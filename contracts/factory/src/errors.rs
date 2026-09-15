use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Error {
    // 1 was `NotInitialized` and 2 was `AlreadyInitialized`, retired when the
    // wasm hash moved into the constructor: a factory can no longer exist
    // uninitialized or be initialized twice. Do not reuse either code.
    // 3 was `NotDeployer`, retired when deployment became permissionless.
    // Do not reuse it: existing clients may still decode 3 as that error.
    /// Org id does not exist.
    OrgNotFound = 4,
    /// The new treasury's `initialize` sub-call failed. Wraps any treasury
    /// error so treasury codes never surface as factory codes.
    TreasuryInitFailed = 5,
    /// The admin deployed an org less than `DEPLOY_COOLDOWN_LEDGERS` ago.
    DeployCooldown = 6,
}
