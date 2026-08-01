use soroban_sdk::{contracttype, Address, String};

/// Storage keys for the factory contract.
///
/// Instance storage holds configuration that rarely changes (deployer, treasury
/// wasm hash, org count). Persistent storage holds the per-org records.
#[contracttype]
pub enum DataKey {
    Deployer,
    WasmHash,
    OrgCount,
    Org(u32),
}

/// A record of a treasury deployed through the factory.
///
/// `treasury` is the deployed contract address, `admin` the treasury's
/// administrative account, and `created_ledger` when the deployment happened.
#[contracttype]
#[derive(Clone)]
pub struct OrgRecord {
    pub name: String,
    pub treasury: Address,
    pub admin: Address,
    pub created_ledger: u32,
}
