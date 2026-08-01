#![no_std]

use soroban_sdk::{contract, contractimpl, panic_with_error, Address, Env, Vec};

mod errors;
mod events;
mod types;

pub use errors::*;
pub use types::*;

/// Charter treasury contract.
///
/// Holds funds under policy: budget categories with spend caps and a
/// multi-signer approval threshold. Disbursements only move through the
/// request/approval flow — there is no admin escape hatch.
#[contract]
pub struct TreasuryContract;

#[contractimpl]
impl TreasuryContract {
    /// Initializes the treasury.
    ///
    /// # Arguments
    /// * `admin` - The address with full administrative control (approvers,
    ///   categories, threshold).
    /// * `approvers` - The addresses authorized to sign off on requests.
    /// * `threshold` - Number of approvals required to execute a request.
    /// * `token` - The token this treasury holds and disburses.
    ///
    /// # Auth
    /// * Requires `admin.require_auth()`.
    ///
    /// # Panics
    /// * `Error::AlreadyInitialized` if already initialized.
    /// * `Error::InvalidThreshold` if `threshold == 0` or exceeds
    ///   `approvers.len()`.
    pub fn initialize(
        env: Env,
        admin: Address,
        approvers: Vec<Address>,
        threshold: u32,
        token: Address,
    ) {
        admin.require_auth();
        if env.storage().instance().has(&DataKey::Admin) {
            panic_with_error!(&env, Error::AlreadyInitialized);
        }
        if threshold == 0 || threshold as usize > approvers.len() as usize {
            panic_with_error!(&env, Error::InvalidThreshold);
        }
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::Approvers, &approvers);
        env.storage().instance().set(&DataKey::Threshold, &threshold);
        env.storage().instance().set(&DataKey::Token, &token);
        env.storage().instance().set(&DataKey::CategoryCount, &0u32);
        env.storage().instance().set(&DataKey::RequestCount, &0u32);
        Self::extend_instance_ttl(&env);
    }
}

impl TreasuryContract {
    fn extend_instance_ttl(env: &Env) {
        env.storage().instance().extend_ttl(100, 100);
    }
}
