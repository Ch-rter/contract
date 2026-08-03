#![no_std]

use soroban_sdk::{contract, contractimpl, panic_with_error, Address, BytesN, Env, String, Vec};

mod errors;
mod events;
mod types;

#[cfg(any(test, feature = "testutils"))]
mod test;

pub use errors::*;
pub use types::*;

use crate::events::TreasuryDeployed;

mod treasury_wasm {
    soroban_sdk::contractimport!(
        file = "../../target/wasm32v1-none/release/charter_treasury.wasm"
    );
}

use treasury_wasm::Client as TreasuryContractClient;

const MAX_ORG_LIMIT: u32 = 50;

/// Charter factory contract.
///
/// Deploys and tracks treasury contracts from an uploaded treasury wasm. The
/// deployer authorizes every deployment; org ids are assigned sequentially and
/// used as the deploy salt so addresses are deterministic.
#[contract]
pub struct FactoryContract;

#[contractimpl]
impl FactoryContract {
    /// Initializes the factory.
    ///
    /// # Arguments
    /// * `deployer` - The address authorized to deploy treasuries.
    /// * `wasm_hash` - Hash of the treasury wasm to deploy.
    ///
    /// # Auth
    /// * Requires `deployer.require_auth()`.
    ///
    /// # Panics
    /// * `Error::AlreadyInitialized` if already initialized.
    pub fn initialize(env: Env, deployer: Address, wasm_hash: BytesN<32>) {
        deployer.require_auth();
        if env.storage().instance().has(&DataKey::Deployer) {
            panic_with_error!(&env, Error::AlreadyInitialized);
        }
        env.storage().instance().set(&DataKey::Deployer, &deployer);
        env.storage().instance().set(&DataKey::WasmHash, &wasm_hash);
        env.storage().instance().set(&DataKey::OrgCount, &0u32);
        Self::extend_instance_ttl(&env);
    }

    /// Deploys a new treasury and initializes it.
    ///
    /// The treasury is deployed with the stored treasury wasm and a salt
    /// derived from the next org id, making its address deterministic. After
    /// deployment the treasury's `initialize` is invoked with the supplied
    /// admin, approvers, threshold and token.
    ///
    /// # Arguments
    /// * `name` - Org name recorded on chain.
    /// * `admin` - Admin of the new treasury.
    /// * `approvers` - Approvers of the new treasury.
    /// * `threshold` - Approval threshold of the new treasury.
    /// * `token` - Token held by the new treasury.
    ///
    /// # Returns
    /// * The newly assigned org id.
    ///
    /// # Auth
    /// * Requires the stored `deployer` to sign (`Error::NotDeployer`
    ///   otherwise).
    /// * Requires `admin` to sign so the treasury's `initialize` sub-call
    ///   succeeds on-chain.
    ///
    /// # Panics
    /// * `Error::NotInitialized` if the factory was not initialized.
    pub fn deploy_treasury(
        env: Env,
        name: String,
        admin: Address,
        approvers: Vec<Address>,
        threshold: u32,
        token: Address,
    ) -> u32 {
        let deployer: Address = env
            .storage()
            .instance()
            .get(&DataKey::Deployer)
            .unwrap_or_else(|| panic_with_error!(&env, Error::NotInitialized));
        deployer.require_auth();
        // The treasury's `initialize` calls `admin.require_auth()`. Requiring
        // the admin's signature at the top invocation ties it to the root call
        // so the sub-call auth succeeds.
        admin.require_auth();
        let wasm_hash: BytesN<32> = env
            .storage()
            .instance()
            .get(&DataKey::WasmHash)
            .unwrap_or_else(|| panic_with_error!(&env, Error::NotInitialized));

        let org_id: u32 = env
            .storage()
            .instance()
            .get(&DataKey::OrgCount)
            .unwrap_or(0)
            + 1;

        let mut salt_bytes = [0u8; 32];
        salt_bytes[0..4].copy_from_slice(&org_id.to_be_bytes());
        let salt = BytesN::from_array(&env, &salt_bytes);

        let treasury =
            env.deployer()
                .with_current_contract(salt)
                .deploy_v2(wasm_hash, ());

        TreasuryContractClient::new(&env, &treasury).initialize(
            &admin,
            &approvers,
            &threshold,
            &token,
        );

        env.storage().instance().set(&DataKey::OrgCount, &org_id);
        env.storage().persistent().set(
            &DataKey::Org(org_id),
            &OrgRecord {
                name: name.clone(),
                treasury: treasury.clone(),
                admin: admin.clone(),
                created_ledger: env.ledger().sequence(),
            },
        );
        env.storage().persistent().extend_ttl(&DataKey::Org(org_id), 100, 100);
        Self::extend_instance_ttl(&env);

        TreasuryDeployed {
            org_id,
            name,
            treasury,
            admin,
        }
        .publish(&env);
        org_id
    }

    /// Returns the org record for the given id.
    ///
    /// # Panics
    /// * `Error::OrgNotFound` if the org does not exist.
    pub fn get_org(env: Env, org_id: u32) -> OrgRecord {
        env.storage()
            .persistent()
            .get(&DataKey::Org(org_id))
            .unwrap_or_else(|| panic_with_error!(&env, Error::OrgNotFound))
    }

    /// Returns the number of orgs deployed so far.
    pub fn get_org_count(env: Env) -> u32 {
        env.storage()
            .instance()
            .get(&DataKey::OrgCount)
            .unwrap_or(0)
    }

    /// Returns a page of org records, starting at `start` (inclusive).
    ///
    /// `limit` is capped at 50.
    pub fn get_orgs(env: Env, start: u32, limit: u32) -> Vec<OrgRecord> {
        let count = Self::get_org_count(env.clone());
        let start = start.max(1);
        let end = (start + limit.min(MAX_ORG_LIMIT)).min(count + 1);
        let mut orgs: Vec<OrgRecord> = Vec::new(&env);
        let mut id = start;
        while id < end {
            orgs.push_back(env.storage().persistent().get(&DataKey::Org(id)).unwrap());
            id += 1;
        }
        orgs
    }

    fn extend_instance_ttl(env: &Env) {
        env.storage().instance().extend_ttl(100, 100);
    }
}
