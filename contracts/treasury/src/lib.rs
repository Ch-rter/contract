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
        if threshold == 0 || threshold > approvers.len() {
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

    /// Adds an approver to the treasury.
    ///
    /// No-op if the address is already an approver.
    ///
    /// # Auth
    /// * Requires the stored `admin` to sign (`Error::NotAdmin` otherwise).
    pub fn add_approver(env: Env, admin: Address, approver: Address) {
        Self::require_admin(&env, &admin);
        let mut approvers: Vec<Address> = env
            .storage()
            .instance()
            .get(&DataKey::Approvers)
            .unwrap_or_else(|| panic_with_error!(&env, Error::NotInitialized));
        if !approvers.contains(&approver) {
            approvers.push_back(approver.clone());
            env.storage().instance().set(&DataKey::Approvers, &approvers);
        }
        Self::extend_instance_ttl(&env);
    }

    /// Removes an approver from the treasury.
    ///
    /// # Auth
    /// * Requires the stored `admin` to sign (`Error::NotAdmin` otherwise).
    ///
    /// # Panics
    /// * `Error::InvalidThreshold` if removal would drop `approvers.len()`
    ///   below `threshold`, which would leave the treasury unable to reach
    ///   quorum.
    pub fn remove_approver(env: Env, admin: Address, approver: Address) {
        Self::require_admin(&env, &admin);
        let mut approvers: Vec<Address> = env
            .storage()
            .instance()
            .get(&DataKey::Approvers)
            .unwrap_or_else(|| panic_with_error!(&env, Error::NotInitialized));
        if let Some(index) = approvers.first_index_of(&approver) {
            approvers.remove(index);
            let threshold: u32 = env
                .storage()
                .instance()
                .get(&DataKey::Threshold)
                .unwrap_or_else(|| panic_with_error!(&env, Error::NotInitialized));
            if approvers.len() < threshold {
                panic_with_error!(&env, Error::InvalidThreshold);
            }
            env.storage().instance().set(&DataKey::Approvers, &approvers);
        }
        Self::extend_instance_ttl(&env);
    }

    /// Updates the approval threshold.
    ///
    /// # Auth
    /// * Requires the stored `admin` to sign (`Error::NotAdmin` otherwise).
    ///
    /// # Panics
    /// * `Error::InvalidThreshold` if `threshold == 0` or exceeds
    ///   `approvers.len()`.
    pub fn set_threshold(env: Env, admin: Address, threshold: u32) {
        Self::require_admin(&env, &admin);
        let approvers: Vec<Address> = env
            .storage()
            .instance()
            .get(&DataKey::Approvers)
            .unwrap_or_else(|| panic_with_error!(&env, Error::NotInitialized));
        if threshold < 1 || threshold > approvers.len() {
            panic_with_error!(&env, Error::InvalidThreshold);
        }
        env.storage().instance().set(&DataKey::Threshold, &threshold);
        Self::extend_instance_ttl(&env);
    }
}

impl TreasuryContract {
    fn extend_instance_ttl(env: &Env) {
        env.storage().instance().extend_ttl(100, 100);
    }

    fn require_admin(env: &Env, admin: &Address) {
        let stored: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .unwrap_or_else(|| panic_with_error!(env, Error::NotInitialized));
        if &stored != admin {
            panic_with_error!(env, Error::NotAdmin);
        }
        admin.require_auth();
    }
}
