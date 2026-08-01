use soroban_sdk::contracterror;

/// Typed errors for the treasury contract.
///
/// Discriminants are explicit and start at 1 (0 is reserved by the host).
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    AlreadyInitialized = 1,
    NotInitialized = 2,
    NotAdmin = 3,
    NotApprover = 4,
    CategoryInactive = 5,
    CapExceeded = 6,
    RequestNotPending = 7,
    InvalidThreshold = 8,
    AlreadyApproved = 9,
    NotRequester = 10,
    InvalidAmount = 11,
}
