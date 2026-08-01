#![no_std]

use soroban_sdk::{contract, contractimpl, panic_with_error, token, Address, Env, MuxedAddress, String, Vec};

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

    /// Creates a new budget category.
    ///
    /// The category starts active with zero spend.
    ///
    /// # Auth
    /// * Requires the stored `admin` to sign (`Error::NotAdmin` otherwise).
    ///
    /// # Panics
    /// * `Error::InvalidAmount` if `cap <= 0`.
    ///
    /// # Returns
    /// * The assigned category id.
    pub fn create_category(env: Env, admin: Address, name: String, cap: i128) -> u32 {
        Self::require_admin(&env, &admin);
        if cap <= 0 {
            panic_with_error!(&env, Error::InvalidAmount);
        }
        let count: u32 = env
            .storage()
            .instance()
            .get(&DataKey::CategoryCount)
            .unwrap_or(0);
        let id = count + 1;
        let category = Category {
            name,
            cap,
            spent: 0,
            active: true,
        };
        let key = DataKey::Category(id);
        env.storage().persistent().set(&key, &category);
        env.storage().persistent().extend_ttl(&key, 100, 100);
        env.storage().instance().set(&DataKey::CategoryCount, &id);
        events::category_created(&env, id, &category.name, cap);
        Self::extend_instance_ttl(&env);
        id
    }

    /// Updates a category's spend cap.
    ///
    /// # Auth
    /// * Requires the stored `admin` to sign (`Error::NotAdmin` otherwise).
    ///
    /// # Panics
    /// * `Error::InvalidAmount` if `new_cap < category.spent` — the cap can
    ///   never be set below what has already been released against it.
    pub fn update_category_cap(env: Env, admin: Address, category_id: u32, new_cap: i128) {
        Self::require_admin(&env, &admin);
        let key = DataKey::Category(category_id);
        let mut category: Category = env
            .storage()
            .persistent()
            .get(&key)
            .unwrap_or_else(|| panic_with_error!(&env, Error::InvalidAmount));
        if new_cap < category.spent {
            panic_with_error!(&env, Error::InvalidAmount);
        }
        category.cap = new_cap;
        env.storage().persistent().set(&key, &category);
        env.storage().persistent().extend_ttl(&key, 100, 100);
        events::category_cap_updated(&env, category_id, new_cap);
        Self::extend_instance_ttl(&env);
    }

    /// Activates or deactivates a category.
    ///
    /// Inactive categories reject new `submit_request` calls; existing
    /// requests remain resolvable. Categories are never deleted.
    ///
    /// # Auth
    /// * Requires the stored `admin` to sign (`Error::NotAdmin` otherwise).
    pub fn set_category_active(env: Env, admin: Address, category_id: u32, active: bool) {
        Self::require_admin(&env, &admin);
        let key = DataKey::Category(category_id);
        let mut category: Category = env
            .storage()
            .persistent()
            .get(&key)
            .unwrap_or_else(|| panic_with_error!(&env, Error::InvalidAmount));
        category.active = active;
        env.storage().persistent().set(&key, &category);
        env.storage().persistent().extend_ttl(&key, 100, 100);
        events::category_active_changed(&env, category_id, active);
        Self::extend_instance_ttl(&env);
    }

    /// Deposits funds into the treasury's general balance.
    ///
    /// Deposits fund the treasury as a whole; categories are spend-side
    /// accounting only, not separate token balances.
    ///
    /// # Auth
    /// * Requires `from.require_auth()`.
    ///
    /// # Panics
    /// * `Error::InvalidAmount` if `amount <= 0`.
    pub fn deposit(env: Env, from: Address, amount: i128) {
        from.require_auth();
        if amount <= 0 {
            panic_with_error!(&env, Error::InvalidAmount);
        }
        let token: Address = env
            .storage()
            .instance()
            .get(&DataKey::Token)
            .unwrap_or_else(|| panic_with_error!(&env, Error::NotInitialized));
        let token_client = token::TokenClient::new(&env, &token);
        token_client.transfer(&from, &MuxedAddress::from(env.current_contract_address()), &amount);
        events::deposited(&env, &from, amount);
        Self::extend_instance_ttl(&env);
    }

    /// Submits a disbursement request against a budget category.
    ///
    /// # Auth
    /// * Requires `requester.require_auth()`.
    ///
    /// # Panics
    /// * `Error::CategoryInactive` if the category is not active.
    /// * `Error::InvalidAmount` if `amount <= 0` or the remaining category cap
    ///   is insufficient.
    ///
    /// # Returns
    /// * The assigned request id.
    pub fn submit_request(
        env: Env,
        requester: Address,
        category_id: u32,
        recipient: Address,
        amount: i128,
        memo: String,
    ) -> u32 {
        requester.require_auth();
        let category_key = DataKey::Category(category_id);
        let category: Category = env
            .storage()
            .persistent()
            .get(&category_key)
            .unwrap_or_else(|| panic_with_error!(&env, Error::InvalidAmount));
        if !category.active {
            panic_with_error!(&env, Error::CategoryInactive);
        }
        if amount <= 0 || category.cap - category.spent < amount {
            panic_with_error!(&env, Error::InvalidAmount);
        }
        let count: u32 = env
            .storage()
            .instance()
            .get(&DataKey::RequestCount)
            .unwrap_or(0);
        let id = count + 1;
        let request = Request {
            id,
            category_id,
            recipient: recipient.clone(),
            amount,
            memo,
            requester: requester.clone(),
            approvals: Vec::new(&env),
            status: RequestStatus::Pending,
            created_ledger: env.ledger().sequence(),
        };
        let request_key = DataKey::Request(id);
        env.storage().persistent().set(&request_key, &request);
        env.storage().persistent().extend_ttl(&request_key, 100, 100);
        env.storage().instance().set(&DataKey::RequestCount, &id);
        events::request_submitted(&env, id, category_id, &recipient, amount);
        Self::extend_instance_ttl(&env);
        id
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
