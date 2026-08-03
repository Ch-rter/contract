use crate::{FactoryContract, FactoryContractClient, TreasuryContractClient};
use soroban_sdk::testutils::{Address as _, Events as _};
use soroban_sdk::{vec, Address, Env, IntoVal, String, Symbol};

/// Deploys a factory with the treasury wasm installed and returns everything
/// the tests need.
struct TestContext {
    env: Env,
    client: FactoryContractClient<'static>,
    deployer: Address,
    admin: Address,
    approver1: Address,
    approver2: Address,
    token: Address,
}

impl TestContext {
    fn new() -> Self {
        let env = Env::default();
        env.mock_all_auths_allowing_non_root_auth();

        let deployer = Address::generate(&env);
        let admin = Address::generate(&env);
        let approver1 = Address::generate(&env);
        let approver2 = Address::generate(&env);

        let token_admin = Address::generate(&env);
        let token_contract = env.register_stellar_asset_contract_v2(token_admin.clone());

        let contract_id = env.register(FactoryContract, ());
        let client = FactoryContractClient::new(&env, &contract_id);

        let wasm_hash = env.deployer().upload_contract_wasm(super::treasury_wasm::WASM);
        client.initialize(&deployer, &wasm_hash);

        TestContext {
            env,
            client,
            deployer,
            admin,
            approver1,
            approver2,
            token: token_contract.address(),
        }
    }
}

fn deploy_org(ctx: &TestContext, name: &str, threshold: u32) -> u32 {
    ctx.client.deploy_treasury(
        &String::from_str(&ctx.env, name),
        &ctx.admin,
        &vec![&ctx.env, ctx.approver1.clone(), ctx.approver2.clone()],
        &threshold,
        &ctx.token,
    )
}

#[test]
fn test_initialize_stores_config() {
    let ctx = TestContext::new();
    assert_eq!(ctx.client.get_org_count(), 0);
    assert_eq!(ctx.client.get_orgs(&0, &10).len(), 0);
}

#[test]
#[should_panic(expected = "Error(Contract, #2)")]
fn test_initialize_twice_panics() {
    let ctx = TestContext::new();
    let wasm_hash = ctx.env.deployer().upload_contract_wasm(super::treasury_wasm::WASM);
    ctx.client.initialize(&ctx.deployer, &wasm_hash);
}

#[test]
#[should_panic(expected = "HostError")]
fn test_initialize_requires_deployer_auth() {
    let env = Env::default();
    env.mock_auths(&[]);
    let deployer = Address::generate(&env);
    let contract_id = env.register(FactoryContract, ());
    let client = FactoryContractClient::new(&env, &contract_id);
    let wasm_hash = env.deployer().upload_contract_wasm(super::treasury_wasm::WASM);
    client.initialize(&deployer, &wasm_hash);
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
    let org1 = ctx.client.get_org(&deploy_org(&ctx, "Charter Org", 2)).treasury;
    let org2 = ctx.client.get_org(&deploy_org(&ctx, "Second Org", 1)).treasury;
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
    for name in names {
        deploy_org(&ctx, name, 1);
    }

    let page = ctx.client.get_orgs(&2, &2);
    assert_eq!(page.len(), 2);
    assert_eq!(page.get(0).unwrap().name, String::from_str(&ctx.env, "Org 1"));
    assert_eq!(page.get(1).unwrap().name, String::from_str(&ctx.env, "Org 2"));

    let tail = ctx.client.get_orgs(&4, &100);
    assert_eq!(tail.len(), 2);
}

#[test]
#[should_panic(expected = "HostError")]
fn test_deploy_treasury_not_deployer_panics() {
    let env = Env::default();
    env.mock_all_auths();
    let deployer = Address::generate(&env);
    let admin = Address::generate(&env);
    let approver1 = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract_v2(token_admin.clone());

    let contract_id = env.register(FactoryContract, ());
    let client = FactoryContractClient::new(&env, &contract_id);
    let wasm_hash = env.deployer().upload_contract_wasm(super::treasury_wasm::WASM);
    client.initialize(&deployer, &wasm_hash);

    // Disable all auths: deployer.require_auth() has no matching signature.
    env.mock_auths(&[]);
    client.deploy_treasury(
        &String::from_str(&env, "Charter Org"),
        &admin,
        &vec![&env, approver1.clone()],
        &1,
        &token_contract.address(),
    );
}

#[test]
#[should_panic(expected = "Error(Contract, #1)")]
fn test_deploy_treasury_not_initialized_panics() {
    let env = Env::default();
    env.mock_all_auths_allowing_non_root_auth();
    let admin = Address::generate(&env);
    let approver1 = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract_v2(token_admin.clone());

    let contract_id = env.register(FactoryContract, ());
    let client = FactoryContractClient::new(&env, &contract_id);
    client.deploy_treasury(
        &String::from_str(&env, "Charter Org"),
        &admin,
        &vec![&env, approver1.clone()],
        &1,
        &token_contract.address(),
    );
}
