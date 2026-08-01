use soroban_sdk::{contractevent, Address, String};

/// Emitted when a new budget category is created.
#[contractevent(data_format = "vec")]
pub struct CategoryCreated {
    #[topic]
    pub category_id: u32,
    pub name: String,
    pub cap: i128,
}

/// Emitted when a category's spend cap changes.
#[contractevent(data_format = "single-value")]
pub struct CapUpdated {
    #[topic]
    pub category_id: u32,
    pub new_cap: i128,
}

/// Emitted when a category is activated or deactivated.
#[contractevent(data_format = "single-value")]
pub struct ActiveChanged {
    #[topic]
    pub category_id: u32,
    pub active: bool,
}

/// Emitted when funds are deposited into the treasury's general balance.
#[contractevent(data_format = "single-value")]
pub struct Deposited {
    #[topic]
    pub from: Address,
    pub amount: i128,
}

/// Emitted when a disbursement request is submitted.
#[contractevent(data_format = "vec")]
pub struct RequestSubmitted {
    #[topic]
    pub request_id: u32,
    pub category_id: u32,
    pub recipient: Address,
    pub amount: i128,
}

/// Emitted when an approver signs off on a pending request.
#[contractevent(data_format = "single-value")]
pub struct RequestApproved {
    #[topic]
    pub request_id: u32,
    pub approver: Address,
}

/// Emitted when a request reaches threshold and funds are released.
#[contractevent(data_format = "vec")]
pub struct RequestExecuted {
    #[topic]
    pub request_id: u32,
    pub recipient: Address,
    pub amount: i128,
}

/// Emitted when a pending request is rejected.
#[contractevent(data_format = "single-value")]
pub struct RequestRejected {
    #[topic]
    pub request_id: u32,
    pub approver: Address,
}

/// Emitted when a pending request is cancelled by its requester.
#[contractevent(data_format = "single-value")]
pub struct RequestCancelled {
    #[topic]
    pub request_id: u32,
}
