use soroban_sdk::{contractevent, Address, String};

/// Emitted when a new treasury is deployed through the factory.
#[contractevent(data_format = "vec")]
pub struct TreasuryDeployed {
    #[topic]
    pub org_id: u32,
    pub name: String,
    pub treasury: Address,
    pub admin: Address,
}
