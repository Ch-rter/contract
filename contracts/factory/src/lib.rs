#![no_std]

use soroban_sdk::{contract, contractimpl};

/// Charter factory contract.
///
/// Deploys and tracks treasury contracts from an uploaded treasury wasm.
#[contract]
pub struct FactoryContract;

#[contractimpl]
impl FactoryContract {}
