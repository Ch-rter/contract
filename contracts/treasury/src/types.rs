use soroban_sdk::{contracttype, Address, String, Vec};

/// Storage keys for the treasury contract.
///
/// Instance storage holds configuration that rarely changes (admin, approvers,
/// threshold, token). Persistent storage holds per-entity data that grows over
/// time (categories, requests).
#[contracttype]
pub enum DataKey {
    Admin,
    Approvers,
    Threshold,
    Token,
    CategoryCount,
    Category(u32),
    RequestCount,
    Request(u32),
}

/// A budget category with a spend cap.
///
/// `cap` is the lifetime ceiling for the category and `spent` tracks how much
/// has already been released against it. Categories are deactivated, never
/// deleted, so historical requests that reference them stay resolvable.
#[contracttype]
#[derive(Clone)]
pub struct Category {
    pub name: String,
    pub cap: i128,
    pub spent: i128,
    pub active: bool,
}

/// Lifecycle state of a disbursement request.
#[contracttype]
#[derive(Clone, PartialEq, Eq)]
pub enum RequestStatus {
    Pending,
    Executed,
    Rejected,
    Cancelled,
}

/// A disbursement request against a budget category.
///
/// `approvals` accumulates the addresses that have signed off. Once the
/// approval count reaches the stored threshold the request auto-executes and
/// funds transfer to `recipient`.
#[contracttype]
#[derive(Clone)]
pub struct Request {
    pub id: u32,
    pub category_id: u32,
    pub recipient: Address,
    pub amount: i128,
    pub memo: String,
    pub requester: Address,
    pub approvals: Vec<Address>,
    pub status: RequestStatus,
    pub created_ledger: u32,
}
