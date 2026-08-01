use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Error {
    /// Factory not initialized: `initialize` has not been called.
    NotInitialized = 1,
    /// Factory already initialized.
    AlreadyInitialized = 2,
    /// Caller is not the deployer authorized to deploy treasuries.
    NotDeployer = 3,
    /// Org id does not exist.
    OrgNotFound = 4,
}
