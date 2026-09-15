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

/// Minimum number of ledgers between two successful `deploy_treasury` calls
/// by the same admin (about one hour at ~5 s per ledger).
pub const DEPLOY_COOLDOWN_LEDGERS: u32 = 720;

/// Charter factory contract.
///
/// Deploys and tracks treasury contracts from an uploaded treasury wasm, whose
/// hash is fixed by the constructor when the factory itself is created.
/// Deployment is permissionless: any account can create an org by signing as
/// that org's own `admin`, and each admin can deploy at most once per
/// `DEPLOY_COOLDOWN_LEDGERS`. Org ids are assigned sequentially and used as the
/// deploy salt so addresses are deterministic.
#[contract]
pub struct FactoryContract;

#[contractimpl]
impl FactoryContract {
    /// Creates the factory and binds it to the treasury wasm it deploys.
    ///
    /// Runs exactly once, inside the transaction that creates the contract, so
    /// the wasm hash is set atomically at deploy. The host rejects any later
    /// call to `__constructor`, and no other function writes the hash, so
    /// nobody can set or replace it after deployment.
    ///
    /// # Arguments
    /// * `wasm_hash` - Hash of the treasury wasm to deploy. Upload that wasm
    ///   first; the constructor does not check that it exists.
    ///
    /// # Auth
    /// * None. Whoever creates the contract chooses the hash, as part of the
    ///   creation itself.
    pub fn __constructor(env: Env, wasm_hash: BytesN<32>) {
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
    /// * Requires `admin` to sign, and nothing else: any account can create an
    ///   org it administers. The same signature covers the treasury's
    ///   `initialize` sub-call, which also requires `admin`.
    ///
    /// # Rate limit
    /// * Each `admin` can deploy at most once per `DEPLOY_COOLDOWN_LEDGERS`.
    ///   A deploy that fails does not start the cooldown.
    ///
    /// # Panics
    /// * `Error::DeployCooldown` if `admin` deployed an org less than
    ///   `DEPLOY_COOLDOWN_LEDGERS` ledgers ago.
    /// * `Error::TreasuryInitFailed` if the treasury's `initialize` fails for
    ///   any reason (e.g. an invalid threshold). The treasury's own error code
    ///   is not propagated.
    pub fn deploy_treasury(
        env: Env,
        name: String,
        admin: Address,
        approvers: Vec<Address>,
        threshold: u32,
        token: Address,
    ) -> u32 {
        // Always present: the constructor sets it when the contract is created.
        let wasm_hash: BytesN<32> = env
            .storage()
            .instance()
            .get(&DataKey::WasmHash)
            .unwrap();
        // The org's own admin is the only required signer. The treasury's
        // `initialize` also calls `admin.require_auth()`; requiring it at the
        // top invocation ties it to the root call so the sub-call auth succeeds.
        admin.require_auth();

        let cooldown_key = DataKey::LastDeploy(admin.clone());
        if let Some(last_deploy) = env.storage().temporary().get::<_, u32>(&cooldown_key) {
            if env.ledger().sequence() < last_deploy.saturating_add(DEPLOY_COOLDOWN_LEDGERS) {
                panic_with_error!(&env, Error::DeployCooldown);
            }
        }

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

        // Treasury and factory error codes overlap with different meanings, so
        // a treasury failure must not propagate as-is. Any failure (contract
        // error, auth or host error) is reported as `TreasuryInitFailed`; the
        // panic rolls back the deployment above along with everything else.
        match TreasuryContractClient::new(&env, &treasury).try_initialize(
            &admin,
            &approvers,
            &threshold,
            &token,
        ) {
            Ok(Ok(())) => {}
            _ => panic_with_error!(&env, Error::TreasuryInitFailed),
        }

        // Start this admin's cooldown. Only reached on success: a failed
        // deploy panics and reverts, so it never locks the admin out. The
        // entry is temporary because it only has to outlive the window.
        env.storage()
            .temporary()
            .set(&cooldown_key, &env.ledger().sequence());
        env.storage().temporary().extend_ttl(
            &cooldown_key,
            DEPLOY_COOLDOWN_LEDGERS,
            DEPLOY_COOLDOWN_LEDGERS,
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
