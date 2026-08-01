use crate::{RequestStatus, TreasuryContract, TreasuryContractClient};
use soroban_sdk::testutils::{Address as _, Events as _};
use soroban_sdk::token::StellarAssetClient;
use soroban_sdk::{vec, Address, Env, IntoVal, String, Symbol, Vec};

/// Deploys a treasury, a test token, and returns everything the tests need.
struct TestContext {
    env: Env,
    client: TreasuryContractClient<'static>,
    admin: Address,
    approver1: Address,
    approver2: Address,
    approver3: Address,
    requester: Address,
    recipient: Address,
    token: Address,
}

impl TestContext {
    fn new(threshold: u32) -> Self {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let approver1 = Address::generate(&env);
        let approver2 = Address::generate(&env);
        let approver3 = Address::generate(&env);
        let requester = Address::generate(&env);
        let recipient = Address::generate(&env);

        let token_admin = Address::generate(&env);
        let token_contract = env.register_stellar_asset_contract_v2(token_admin.clone());
        let token = token_contract.address();

        let contract_id = env.register(TreasuryContract, ());
        let client = TreasuryContractClient::new(&env, &contract_id);

        client.initialize(
            &admin,
            &Vec::from_array(&env, [approver1.clone(), approver2.clone(), approver3.clone()]),
            &threshold,
            &token,
        );

        TestContext {
            env,
            client,
            admin,
            approver1,
            approver2,
            approver3,
            requester,
            recipient,
            token,
        }
    }

    /// Mints `amount` to the treasury so approval execution can transfer it.
    fn fund_treasury(&self, amount: i128) {
        let stellar = StellarAssetClient::new(&self.env, &self.token);
        stellar.mint(&self.client.address, &amount);
    }
}

#[test]
fn test_initialize_stores_config() {
    let ctx = TestContext::new(2);

    assert_eq!(ctx.client.get_approvers(), Vec::from_array(
        &ctx.env,
        [
            ctx.approver1.clone(),
            ctx.approver2.clone(),
            ctx.approver3.clone(),
        ],
    ));
    assert_eq!(ctx.client.get_threshold(), 2);
    assert_eq!(ctx.client.get_categories().len(), 0);
}

#[test]
#[should_panic(expected = "Error(Contract, #1)")]
fn test_initialize_twice_panics() {
    let ctx = TestContext::new(1);
    ctx.client.initialize(
        &ctx.admin,
        &Vec::from_array(&ctx.env, [ctx.approver1.clone()]),
        &1,
        &ctx.token,
    );
}

#[test]
#[should_panic(expected = "Error(Contract, #8)")]
fn test_initialize_zero_threshold_panics() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract_v2(token_admin.clone());
    let contract_id = env.register(TreasuryContract, ());
    let client = TreasuryContractClient::new(&env, &contract_id);
    client.initialize(
        &admin,
        &Vec::from_array(&env, [Address::generate(&env)]),
        &0,
        &token_contract.address(),
    );
}

#[test]
#[should_panic(expected = "Error(Contract, #8)")]
fn test_initialize_threshold_exceeds_approvers_panics() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract_v2(token_admin.clone());
    let contract_id = env.register(TreasuryContract, ());
    let client = TreasuryContractClient::new(&env, &contract_id);
    client.initialize(
        &admin,
        &Vec::from_array(&env, [Address::generate(&env)]),
        &2,
        &token_contract.address(),
    );
}

#[test]
fn test_add_approver_appends_and_is_idempotent() {
    let ctx = TestContext::new(1);
    let new_approver = Address::generate(&ctx.env);
    ctx.client.add_approver(&ctx.admin, &new_approver);
    let approvers = ctx.client.get_approvers();
    assert!(approvers.contains(&new_approver));
    assert_eq!(approvers.len(), 4);

    // Adding the same approver again is a no-op.
    ctx.client.add_approver(&ctx.admin, &new_approver);
    assert_eq!(ctx.client.get_approvers().len(), 4);
}

#[test]
fn test_remove_approver_removes() {
    let ctx = TestContext::new(1);
    ctx.client.remove_approver(&ctx.admin, &ctx.approver3);
    assert!(!ctx.client.get_approvers().contains(&ctx.approver3));
}

#[test]
#[should_panic(expected = "Error(Contract, #8)")]
fn test_remove_approver_below_threshold_panics() {
    let ctx = TestContext::new(2);
    ctx.client.remove_approver(&ctx.admin, &ctx.approver3);
    // threshold is 2, remaining approvers would be 2, still ok
    ctx.client.remove_approver(&ctx.admin, &ctx.approver2);
    // remaining approvers would be 1, below threshold 2 -> panic
}

#[test]
fn test_set_threshold_updates() {
    let ctx = TestContext::new(1);
    ctx.client.set_threshold(&ctx.admin, &2);
    assert_eq!(ctx.client.get_threshold(), 2);
}

#[test]
#[should_panic(expected = "Error(Contract, #8)")]
fn test_set_threshold_zero_panics() {
    let ctx = TestContext::new(1);
    ctx.client.set_threshold(&ctx.admin, &0);
}

#[test]
#[should_panic(expected = "Error(Contract, #8)")]
fn test_set_threshold_above_approver_count_panics() {
    let ctx = TestContext::new(1);
    ctx.client.set_threshold(&ctx.admin, &4);
}

#[test]
fn test_create_category_returns_id_and_emits_event() {
    let ctx = TestContext::new(1);
    let name = String::from_str(&ctx.env, "Engineering");
    let id = ctx.client.create_category(&ctx.admin, &name, &10_000);
    assert_eq!(id, 1);

    let events = ctx.env.events().all();
    assert_eq!(
        events,
        vec![
            &ctx.env,
            (
                ctx.client.address.clone(),
                (Symbol::new(&ctx.env, "category_created"), 1u32).into_val(&ctx.env),
                (name.clone(), 10_000i128).into_val(&ctx.env),
            ),
        ]
    );

    let category = ctx.client.get_category(&id);
    assert_eq!(category.name, name);
    assert_eq!(category.cap, 10_000);
    assert_eq!(category.spent, 0);
    assert!(category.active);
}

#[test]
fn test_get_categories_returns_all() {
    let ctx = TestContext::new(1);
    ctx.client.create_category(&ctx.admin, &String::from_str(&ctx.env, "Ops"), &5_000);
    ctx.client.create_category(&ctx.admin, &String::from_str(&ctx.env, "Growth"), &20_000);
    let categories = ctx.client.get_categories();
    assert_eq!(categories.len(), 2);
    assert_eq!(categories.get(0).unwrap().name, String::from_str(&ctx.env, "Ops"));
    assert_eq!(categories.get(1).unwrap().cap, 20_000);
}

#[test]
fn test_update_category_cap() {
    let ctx = TestContext::new(1);
    let id = ctx.client.create_category(&ctx.admin, &String::from_str(&ctx.env, "Ops"), &5_000);
    ctx.client.update_category_cap(&ctx.admin, &id, &8_000);
    assert_eq!(ctx.client.get_category(&id).cap, 8_000);
}

#[test]
#[should_panic(expected = "Error(Contract, #11)")]
fn test_update_category_cap_below_spent_panics() {
    let ctx = TestContext::new(1);
    let id = ctx.client.create_category(&ctx.admin, &String::from_str(&ctx.env, "Ops"), &5_000);
    // Force spent above 0 by executing a request, then try to drop cap below it.
    ctx.fund_treasury(5_000);
    let req_id = ctx.client.submit_request(
        &ctx.requester,
        &id,
        &ctx.recipient,
        &1_000,
        &String::from_str(&ctx.env, "stipend"),
    );
    ctx.client.approve_request(&ctx.approver1, &req_id);
    // spent is now 1_000; a cap of 500 would be below spent -> panic
    ctx.client.update_category_cap(&ctx.admin, &id, &500);
}

#[test]
fn test_set_category_active_flips_flag() {
    let ctx = TestContext::new(1);
    let id = ctx.client.create_category(&ctx.admin, &String::from_str(&ctx.env, "Ops"), &5_000);
    ctx.client.set_category_active(&ctx.admin, &id, &false);
    assert!(!ctx.client.get_category(&id).active);
    ctx.client.set_category_active(&ctx.admin, &id, &true);
    assert!(ctx.client.get_category(&id).active);
}

#[test]
fn test_submit_request_returns_id_and_stores_request() {
    let ctx = TestContext::new(2);
    let id = ctx.client.create_category(&ctx.admin, &String::from_str(&ctx.env, "Ops"), &5_000);
    let req_id = ctx.client.submit_request(
        &ctx.requester,
        &id,
        &ctx.recipient,
        &1_000,
        &String::from_str(&ctx.env, "stipend"),
    );
    assert_eq!(req_id, 1);

    let request = ctx.client.get_request(&req_id);
    assert_eq!(request.id, req_id);
    assert_eq!(request.category_id, id);
    assert_eq!(request.recipient, ctx.recipient);
    assert_eq!(request.amount, 1_000);
    assert_eq!(request.memo, String::from_str(&ctx.env, "stipend"));
    assert_eq!(request.requester, ctx.requester);
    assert_eq!(request.approvals.len(), 0);
    assert_eq!(request.status, RequestStatus::Pending);
}

#[test]
#[should_panic(expected = "Error(Contract, #5)")]
fn test_submit_request_inactive_category_panics() {
    let ctx = TestContext::new(1);
    let id = ctx.client.create_category(&ctx.admin, &String::from_str(&ctx.env, "Ops"), &5_000);
    ctx.client.set_category_active(&ctx.admin, &id, &false);
    ctx.client.submit_request(
        &ctx.requester,
        &id,
        &ctx.recipient,
        &1_000,
        &String::from_str(&ctx.env, "stipend"),
    );
}

#[test]
#[should_panic(expected = "Error(Contract, #11)")]
fn test_submit_request_zero_amount_panics() {
    let ctx = TestContext::new(1);
    let id = ctx.client.create_category(&ctx.admin, &String::from_str(&ctx.env, "Ops"), &5_000);
    ctx.client.submit_request(
        &ctx.requester,
        &id,
        &ctx.recipient,
        &0,
        &String::from_str(&ctx.env, "stipend"),
    );
}

#[test]
#[should_panic(expected = "Error(Contract, #11)")]
fn test_submit_request_cap_exceeded_panics() {
    let ctx = TestContext::new(1);
    let id = ctx.client.create_category(&ctx.admin, &String::from_str(&ctx.env, "Ops"), &5_000);
    ctx.client.submit_request(
        &ctx.requester,
        &id,
        &ctx.recipient,
        &5_001,
        &String::from_str(&ctx.env, "stipend"),
    );
}

#[test]
fn test_get_requests_by_category_filters() {
    let ctx = TestContext::new(1);
    let ops = ctx.client.create_category(&ctx.admin, &String::from_str(&ctx.env, "Ops"), &5_000);
    let growth =
        ctx.client.create_category(&ctx.admin, &String::from_str(&ctx.env, "Growth"), &20_000);
    let req1 = ctx.client.submit_request(
        &ctx.requester,
        &ops,
        &ctx.recipient,
        &1_000,
        &String::from_str(&ctx.env, "a"),
    );
    let req2 = ctx.client.submit_request(
        &ctx.requester,
        &growth,
        &ctx.recipient,
        &2_000,
        &String::from_str(&ctx.env, "b"),
    );
    let req3 = ctx.client.submit_request(
        &ctx.requester,
        &ops,
        &ctx.recipient,
        &3_000,
        &String::from_str(&ctx.env, "c"),
    );

    let ops_requests = ctx.client.get_requests_by_category(&ops);
    assert_eq!(ops_requests.len(), 2);
    assert_eq!(ops_requests.get(0).unwrap().id, req1);
    assert_eq!(ops_requests.get(1).unwrap().id, req3);

    let growth_requests = ctx.client.get_requests_by_category(&growth);
    assert_eq!(growth_requests.len(), 1);
    assert_eq!(growth_requests.get(0).unwrap().id, req2);
}

#[test]
fn test_approve_request_accumulates_until_threshold() {
    let ctx = TestContext::new(2);
    let id = ctx.client.create_category(&ctx.admin, &String::from_str(&ctx.env, "Ops"), &5_000);
    let req_id = ctx.client.submit_request(
        &ctx.requester,
        &id,
        &ctx.recipient,
        &1_000,
        &String::from_str(&ctx.env, "stipend"),
    );

    // First approval: below threshold of 2, request stays pending.
    ctx.client.approve_request(&ctx.approver1, &req_id);
    let request = ctx.client.get_request(&req_id);
    assert_eq!(request.approvals.len(), 1);
    assert_eq!(request.approvals.get(0).unwrap(), ctx.approver1);
    assert_eq!(request.status, RequestStatus::Pending);
}

#[test]
fn test_approve_request_auto_executes_at_threshold() {
    let ctx = TestContext::new(2);
    ctx.fund_treasury(5_000);
    let id = ctx.client.create_category(&ctx.admin, &String::from_str(&ctx.env, "Ops"), &5_000);
    let req_id = ctx.client.submit_request(
        &ctx.requester,
        &id,
        &ctx.recipient,
        &1_000,
        &String::from_str(&ctx.env, "stipend"),
    );

    let recipient_before = StellarAssetClient::new(&ctx.env, &ctx.token).balance(&ctx.recipient);
    ctx.client.approve_request(&ctx.approver1, &req_id);
    ctx.client.approve_request(&ctx.approver2, &req_id);

    let request = ctx.client.get_request(&req_id);
    assert_eq!(request.status, RequestStatus::Executed);
    assert_eq!(request.approvals.len(), 2);

    // Recipient received the funds and the category spent is incremented.
    let recipient_after = StellarAssetClient::new(&ctx.env, &ctx.token).balance(&ctx.recipient);
    assert_eq!(recipient_after - recipient_before, 1_000);
    assert_eq!(ctx.client.get_category(&id).spent, 1_000);
}

#[test]
#[should_panic(expected = "Error(Contract, #4)")]
fn test_approve_request_not_approver_panics() {
    let ctx = TestContext::new(1);
    let id = ctx.client.create_category(&ctx.admin, &String::from_str(&ctx.env, "Ops"), &5_000);
    let req_id = ctx.client.submit_request(
        &ctx.requester,
        &id,
        &ctx.recipient,
        &1_000,
        &String::from_str(&ctx.env, "stipend"),
    );
    ctx.client.approve_request(&ctx.requester, &req_id);
}

#[test]
#[should_panic(expected = "Error(Contract, #9)")]
fn test_approve_request_twice_panics() {
    let ctx = TestContext::new(2);
    let id = ctx.client.create_category(&ctx.admin, &String::from_str(&ctx.env, "Ops"), &5_000);
    let req_id = ctx.client.submit_request(
        &ctx.requester,
        &id,
        &ctx.recipient,
        &1_000,
        &String::from_str(&ctx.env, "stipend"),
    );
    ctx.client.approve_request(&ctx.approver1, &req_id);
    ctx.client.approve_request(&ctx.approver1, &req_id);
}

#[test]
#[should_panic(expected = "Error(Contract, #7)")]
fn test_approve_request_not_pending_panics() {
    let ctx = TestContext::new(1);
    ctx.fund_treasury(5_000);
    let id = ctx.client.create_category(&ctx.admin, &String::from_str(&ctx.env, "Ops"), &5_000);
    let req_id = ctx.client.submit_request(
        &ctx.requester,
        &id,
        &ctx.recipient,
        &1_000,
        &String::from_str(&ctx.env, "stipend"),
    );
    ctx.client.approve_request(&ctx.approver1, &req_id);
    // Already executed; approving again must panic.
    ctx.client.approve_request(&ctx.approver2, &req_id);
}

#[test]
fn test_reject_request_sets_status() {
    let ctx = TestContext::new(2);
    let id = ctx.client.create_category(&ctx.admin, &String::from_str(&ctx.env, "Ops"), &5_000);
    let req_id = ctx.client.submit_request(
        &ctx.requester,
        &id,
        &ctx.recipient,
        &1_000,
        &String::from_str(&ctx.env, "stipend"),
    );
    ctx.client.reject_request(&ctx.approver1, &req_id);
    assert_eq!(ctx.client.get_request(&req_id).status, RequestStatus::Rejected);
}

#[test]
#[should_panic(expected = "Error(Contract, #4)")]
fn test_reject_request_not_approver_panics() {
    let ctx = TestContext::new(1);
    let id = ctx.client.create_category(&ctx.admin, &String::from_str(&ctx.env, "Ops"), &5_000);
    let req_id = ctx.client.submit_request(
        &ctx.requester,
        &id,
        &ctx.recipient,
        &1_000,
        &String::from_str(&ctx.env, "stipend"),
    );
    ctx.client.reject_request(&ctx.requester, &req_id);
}

#[test]
#[should_panic(expected = "Error(Contract, #7)")]
fn test_reject_request_not_pending_panics() {
    let ctx = TestContext::new(1);
    let id = ctx.client.create_category(&ctx.admin, &String::from_str(&ctx.env, "Ops"), &5_000);
    let req_id = ctx.client.submit_request(
        &ctx.requester,
        &id,
        &ctx.recipient,
        &1_000,
        &String::from_str(&ctx.env, "stipend"),
    );
    ctx.client.reject_request(&ctx.approver1, &req_id);
    ctx.client.reject_request(&ctx.approver2, &req_id);
}

#[test]
fn test_cancel_request_by_requester() {
    let ctx = TestContext::new(1);
    let id = ctx.client.create_category(&ctx.admin, &String::from_str(&ctx.env, "Ops"), &5_000);
    let req_id = ctx.client.submit_request(
        &ctx.requester,
        &id,
        &ctx.recipient,
        &1_000,
        &String::from_str(&ctx.env, "stipend"),
    );
    ctx.client.cancel_request(&ctx.requester, &req_id);
    assert_eq!(ctx.client.get_request(&req_id).status, RequestStatus::Cancelled);
}

#[test]
#[should_panic(expected = "Error(Contract, #10)")]
fn test_cancel_request_not_requester_panics() {
    let ctx = TestContext::new(1);
    let id = ctx.client.create_category(&ctx.admin, &String::from_str(&ctx.env, "Ops"), &5_000);
    let req_id = ctx.client.submit_request(
        &ctx.requester,
        &id,
        &ctx.recipient,
        &1_000,
        &String::from_str(&ctx.env, "stipend"),
    );
    ctx.client.cancel_request(&ctx.approver1, &req_id);
}

#[test]
#[should_panic(expected = "Error(Contract, #7)")]
fn test_cancel_request_not_pending_panics() {
    let ctx = TestContext::new(1);
    let id = ctx.client.create_category(&ctx.admin, &String::from_str(&ctx.env, "Ops"), &5_000);
    let req_id = ctx.client.submit_request(
        &ctx.requester,
        &id,
        &ctx.recipient,
        &1_000,
        &String::from_str(&ctx.env, "stipend"),
    );
    ctx.client.cancel_request(&ctx.requester, &req_id);
    ctx.client.cancel_request(&ctx.requester, &req_id);
}

#[test]
fn test_get_balance_reflects_funding() {
    let ctx = TestContext::new(1);
    assert_eq!(ctx.client.get_balance(), 0);
    ctx.fund_treasury(5_000);
    assert_eq!(ctx.client.get_balance(), 5_000);
}
