use crate::{
    Error, FactoryContract, FactoryContractArgs, FactoryContractClient, TreasuryContractClient,
    DEPLOY_COOLDOWN_LEDGERS,
};
use soroban_sdk::testutils::{Address as _, Events as _, Ledger as _};
use soroban_sdk::{
    vec, Address, BytesN, ConversionError, Env, IntoVal, InvokeError, String, Symbol, Val,
};

/// Deploys a factory with the treasury wasm installed and returns everything
/// the tests need.
struct TestContext {
    env: Env,
    client: FactoryContractClient<'static>,
    admin: Address,
    approver1: Address,
    approver2: Address,
    token: Address,
}

impl TestContext {
    fn new() -> Self {
        let env = Env::default();
        // Keep contract, wasm and org entries live across the ledger jumps the
        // cooldown tests make (the factory only extends its own TTL to 100).
        env.ledger().with_mut(|li| {
            li.min_persistent_entry_ttl = 10 * DEPLOY_COOLDOWN_LEDGERS;
            li.max_entry_ttl = 100 * DEPLOY_COOLDOWN_LEDGERS;
        });
        env.mock_all_auths_allowing_non_root_auth();

        let admin = Address::generate(&env);
        let approver1 = Address::generate(&env);
        let approver2 = Address::generate(&env);

        let token_admin = Address::generate(&env);
        let token_contract = env.register_stellar_asset_contract_v2(token_admin.clone());

        // The wasm hash is a constructor argument: the factory is bound to it
        // in the same step that creates the contract.
        let wasm_hash = env.deployer().upload_contract_wasm(super::treasury_wasm::WASM);
        let contract_id =
            env.register(FactoryContract, FactoryContractArgs::__constructor(&wasm_hash));
        let client = FactoryContractClient::new(&env, &contract_id);

        TestContext {
            env,
            client,
            admin,
            approver1,
            approver2,
            token: token_contract.address(),
        }
    }
}

fn deploy_org(ctx: &TestContext, name: &str, threshold: u32) -> u32 {
    deploy_org_as(ctx, &ctx.admin, name, threshold)
}

fn deploy_org_as(ctx: &TestContext, admin: &Address, name: &str, threshold: u32) -> u32 {
    ctx.client.deploy_treasury(
        &String::from_str(&ctx.env, name),
        admin,
        &vec![&ctx.env, ctx.approver1.clone(), ctx.approver2.clone()],
        &threshold,
        &ctx.token,
    )
}

fn try_deploy_org_as(
    ctx: &TestContext,
    admin: &Address,
    name: &str,
    threshold: u32,
) -> Result<Result<u32, ConversionError>, Result<soroban_sdk::Error, InvokeError>> {
    ctx.client.try_deploy_treasury(
        &String::from_str(&ctx.env, name),
        admin,
        &vec![&ctx.env, ctx.approver1.clone(), ctx.approver2.clone()],
        &threshold,
        &ctx.token,
    )
}

/// Tries to set the factory's treasury wasm hash through `func` after the
/// factory has been deployed, the way a front-runner would, with every
/// authorization mocked in the attacker's favour. Returns whether it worked.
fn front_run_set_wasm_hash(ctx: &TestContext, func: &str) -> bool {
    let attacker_hash = BytesN::from_array(&ctx.env, &[7u8; 32]);
    let args: soroban_sdk::Vec<Val> = vec![&ctx.env, attacker_hash.into_val(&ctx.env)];
    ctx.env
        .try_invoke_contract::<(), soroban_sdk::Error>(
            &ctx.client.address,
            &Symbol::new(&ctx.env, func),
            args,
        )
        .is_ok()
}

/// Proves the stored hash is still the real treasury wasm. No wasm with the
/// attacker's hash was uploaded, so a replaced hash would make this deploy
/// fail instead of producing a working treasury.
fn assert_deploys_real_treasury(ctx: &TestContext) {
    let org = ctx.client.get_org(&deploy_org(ctx, "Charter Org", 2));
    assert_eq!(TreasuryContractClient::new(&ctx.env, &org.treasury).get_threshold(), 2);
}

#[test]
fn test_constructor_binds_wasm_hash_at_deploy() {
    // Creating the factory runs the constructor, so it can deploy treasuries
    // straight away: there is no separate initialization step to race.
    let ctx = TestContext::new();
    assert_eq!(ctx.client.get_org_count(), 0);
    assert_eq!(ctx.client.get_orgs(&0, &10).len(), 0);
    assert_deploys_real_treasury(&ctx);
}

#[test]
fn test_front_run_cannot_set_wasm_hash_via_initialize() {
    let ctx = TestContext::new();
    // The old two-step flow's public setter no longer exists.
    assert!(!front_run_set_wasm_hash(&ctx, "initialize"));
    assert_deploys_real_treasury(&ctx);
}

#[test]
fn test_front_run_cannot_rerun_constructor_after_deploy() {
    let ctx = TestContext::new();
    // The host only runs `__constructor` while creating the contract.
    assert!(!front_run_set_wasm_hash(&ctx, "__constructor"));
    assert_deploys_real_treasury(&ctx);
}

#[test]
#[should_panic]
fn test_factory_cannot_be_created_without_wasm_hash() {
    // The constructor requires the hash, so creating the contract without it
    // fails: an unbound factory never exists, even briefly.
    let env = Env::default();
    env.register(FactoryContract, ());
}

#[test]
fn test_deploy_treasury_assigns_id_and_emits_event() {
    let ctx = TestContext::new();
    let name = String::from_str(&ctx.env, "Charter Org");

    let org_id = ctx.client.deploy_treasury(
        &name,
        &ctx.admin,
        &vec![&ctx.env, ctx.approver1.clone(), ctx.approver2.clone()],
        &2,
        &ctx.token,
    );
    assert_eq!(org_id, 1);

    // The treasury address is deterministic: salt derived from org id.
    let mut salt_bytes = [0u8; 32];
    salt_bytes[0..4].copy_from_slice(&1u32.to_be_bytes());
    let salt = soroban_sdk::BytesN::from_array(&ctx.env, &salt_bytes);
    let treasury = ctx
        .env
        .deployer()
        .with_address(ctx.client.address.clone(), salt)
        .deployed_address();

    let events = ctx.env.events().all();
    assert_eq!(
        events,
        vec![
            &ctx.env,
            (
                ctx.client.address.clone(),
                (Symbol::new(&ctx.env, "treasury_deployed"), 1u32).into_val(&ctx.env),
                (name.clone(), treasury.clone(), ctx.admin.clone()).into_val(&ctx.env),
            ),
        ]
    );

    assert_eq!(ctx.client.get_org_count(), 1);
    assert_eq!(ctx.client.get_org(&1).treasury, treasury);
}

#[test]
fn test_deploy_treasury_initializes_treasury() {
    let ctx = TestContext::new();
    let org_id = deploy_org(&ctx, "Charter Org", 2);

    let org = ctx.client.get_org(&org_id);
    let treasury_client = TreasuryContractClient::new(&ctx.env, &org.treasury);
    assert_eq!(treasury_client.get_threshold(), 2);
    assert_eq!(
        treasury_client.get_approvers(),
        vec![&ctx.env, ctx.approver1.clone(), ctx.approver2.clone()]
    );
}

#[test]
fn test_deploy_treasury_addresses_are_distinct() {
    let ctx = TestContext::new();
    let second_admin = Address::generate(&ctx.env);
    let org1 = ctx.client.get_org(&deploy_org(&ctx, "Charter Org", 2)).treasury;
    let org2 = ctx.client.get_org(&deploy_org_as(&ctx, &second_admin, "Second Org", 1)).treasury;
    assert_ne!(org1, org2);
}

#[test]
fn test_get_org_returns_record() {
    let ctx = TestContext::new();
    let name = String::from_str(&ctx.env, "Charter Org");
    let org_id = ctx.client.deploy_treasury(
        &name,
        &ctx.admin,
        &vec![&ctx.env, ctx.approver1.clone(), ctx.approver2.clone()],
        &2,
        &ctx.token,
    );

    let org = ctx.client.get_org(&org_id);
    assert_eq!(org.name, name);
    assert_eq!(org.admin, ctx.admin);
    assert_eq!(org.created_ledger, ctx.env.ledger().sequence());
}

#[test]
#[should_panic(expected = "Error(Contract, #4)")]
fn test_get_org_unknown_panics() {
    let ctx = TestContext::new();
    ctx.client.get_org(&99);
}

#[test]
fn test_get_orgs_paginates() {
    let ctx = TestContext::new();
    let names = ["Org 0", "Org 1", "Org 2", "Org 3", "Org 4"];
    // One admin per org: a single admin is rate limited by the cooldown.
    for name in names {
        let admin = Address::generate(&ctx.env);
        deploy_org_as(&ctx, &admin, name, 1);
    }

    let page = ctx.client.get_orgs(&2, &2);
    assert_eq!(page.len(), 2);
    assert_eq!(page.get(0).unwrap().name, String::from_str(&ctx.env, "Org 1"));
    assert_eq!(page.get(1).unwrap().name, String::from_str(&ctx.env, "Org 2"));

    let tail = ctx.client.get_orgs(&4, &100);
    assert_eq!(tail.len(), 2);
}

#[test]
fn test_deploy_treasury_any_account_can_deploy_as_own_admin() {
    let ctx = TestContext::new();
    // An account with no part in the factory's setup creates an org it
    // administers. Strict (root-only) auth mocking, so every signature the
    // call needs is recorded below.
    let org_admin = Address::generate(&ctx.env);
    ctx.env.mock_all_auths();

    let org_id = deploy_org_as(&ctx, &org_admin, "Community Org", 2);

    // The org's own admin is the only address that had to authorize.
    let auths = ctx.env.auths();
    assert_eq!(auths.len(), 1);
    assert_eq!(auths[0].0, org_admin);

    let org = ctx.client.get_org(&org_id);
    assert_eq!(org.admin, org_admin);
    assert_eq!(TreasuryContractClient::new(&ctx.env, &org.treasury).get_threshold(), 2);
}

#[test]
#[should_panic(expected = "Error(Auth, InvalidAction)")]
fn test_deploy_treasury_requires_admin_auth() {
    let ctx = TestContext::new();
    // No signatures: the admin's authorization is the gate and must fail.
    ctx.env.mock_auths(&[]);
    deploy_org(&ctx, "Charter Org", 2);
}

#[test]
fn test_deploy_treasury_cooldown_blocks_same_admin_until_elapsed() {
    let ctx = TestContext::new();
    let first_ledger = ctx.env.ledger().sequence();
    assert_eq!(deploy_org(&ctx, "First Org", 2), 1);

    // Same admin, same ledger: rejected.
    assert_eq!(
        try_deploy_org_as(&ctx, &ctx.admin, "Spam Org", 2),
        Err(Ok(Error::DeployCooldown.into()))
    );

    // Last ledger inside the window: still rejected.
    ctx.env
        .ledger()
        .set_sequence_number(first_ledger + DEPLOY_COOLDOWN_LEDGERS - 1);
    assert_eq!(
        try_deploy_org_as(&ctx, &ctx.admin, "Spam Org", 2),
        Err(Ok(Error::DeployCooldown.into()))
    );
    assert_eq!(ctx.client.get_org_count(), 1);

    // First ledger after the window: allowed again.
    ctx.env
        .ledger()
        .set_sequence_number(first_ledger + DEPLOY_COOLDOWN_LEDGERS);
    assert_eq!(deploy_org(&ctx, "Second Org", 2), 2);
}

#[test]
fn test_deploy_treasury_cooldown_is_per_admin() {
    let ctx = TestContext::new();
    assert_eq!(deploy_org(&ctx, "First Org", 2), 1);

    // A different admin in the same ledger is not affected.
    let other_admin = Address::generate(&ctx.env);
    assert_eq!(deploy_org_as(&ctx, &other_admin, "Other Org", 2), 2);
}

#[test]
fn test_deploy_treasury_failed_deploy_does_not_start_cooldown() {
    let ctx = TestContext::new();
    // Threshold above the approver count fails treasury initialization.
    assert_eq!(
        try_deploy_org_as(&ctx, &ctx.admin, "Bad Org", 3),
        Err(Ok(Error::TreasuryInitFailed.into()))
    );

    // The failure reverted, so the same admin can deploy straight away.
    assert_eq!(deploy_org(&ctx, "Good Org", 2), 1);
}

#[test]
fn test_deploy_treasury_invalid_threshold_returns_factory_error() {
    let ctx = TestContext::new();
    // Threshold above the approver count: the treasury rejects it with its own
    // InvalidThreshold (#8), which must surface as the factory's error instead.
    let result = try_deploy_org_as(&ctx, &ctx.admin, "Charter Org", 3);
    assert_eq!(result, Err(Ok(Error::TreasuryInitFailed.into())));
    assert_eq!(ctx.client.get_org_count(), 0);
}

#[test]
#[should_panic(expected = "Error(Contract, #5)")]
fn test_deploy_treasury_zero_threshold_panics() {
    let ctx = TestContext::new();
    deploy_org(&ctx, "Charter Org", 0);
}
