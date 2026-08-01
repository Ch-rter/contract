use soroban_sdk::{Address, Env, String, Symbol};

/// Emitted when a new budget category is created.
pub fn category_created(env: &Env, category_id: u32, name: &String, cap: i128) {
    env.events().publish(
        (Symbol::new(env, "category_created"), category_id),
        (name.clone(), cap),
    );
}

/// Emitted when a category's spend cap changes.
pub fn category_cap_updated(env: &Env, category_id: u32, new_cap: i128) {
    env.events()
        .publish((Symbol::new(env, "cap_updated"), category_id), new_cap);
}

/// Emitted when a category is activated or deactivated.
pub fn category_active_changed(env: &Env, category_id: u32, active: bool) {
    env.events().publish(
        (Symbol::new(env, "active_changed"), category_id),
        active,
    );
}

/// Emitted when funds are deposited into the treasury's general balance.
pub fn deposited(env: &Env, from: &Address, amount: i128) {
    env.events()
        .publish((Symbol::new(env, "deposited"), from.clone()), amount);
}

/// Emitted when a disbursement request is submitted.
pub fn request_submitted(
    env: &Env,
    request_id: u32,
    category_id: u32,
    recipient: &Address,
    amount: i128,
) {
    env.events().publish(
        (Symbol::new(env, "request_submitted"), request_id),
        (category_id, recipient.clone(), amount),
    );
}

/// Emitted when an approver signs off on a pending request.
pub fn request_approved(env: &Env, request_id: u32, approver: &Address) {
    env.events().publish(
        (Symbol::new(env, "request_approved"), request_id),
        approver.clone(),
    );
}

/// Emitted when a request reaches threshold and funds are released.
pub fn request_executed(env: &Env, request_id: u32, recipient: &Address, amount: i128) {
    env.events().publish(
        (Symbol::new(env, "request_executed"), request_id),
        (recipient.clone(), amount),
    );
}

/// Emitted when a pending request is rejected.
pub fn request_rejected(env: &Env, request_id: u32, approver: &Address) {
    env.events().publish(
        (Symbol::new(env, "request_rejected"), request_id),
        approver.clone(),
    );
}

/// Emitted when a pending request is cancelled by its requester.
pub fn request_cancelled(env: &Env, request_id: u32) {
    env.events()
        .publish((Symbol::new(env, "request_cancelled"), request_id), ());
}
