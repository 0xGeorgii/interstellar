// lib.rs
#![no_std]
use soroban_sdk::{
    contract, contractimpl, contractmeta, contracttype, panic_with_error, symbol_short, xdr::ToXdr, Address, BytesN, Env, Symbol, Vec
};

contractmeta!(
    key = "Description",
    val = "Bare-bone cross-chain atomic swap escrow factory"
);

#[derive(Clone)]
#[contracttype]
pub struct EscrowImmutables {
    pub order_hash: BytesN<32>,
    pub hashlock: BytesN<32>,  // Hash of the secret
    pub maker: Address,
    pub taker: Address,
    pub token: Address,
    pub amount: i128,
    pub safety_deposit: i128,
    pub timelocks: TimeLocks,  // Timelocks for withdrawal and cancellation
}

#[derive(Clone)]
#[contracttype]
pub struct TimeLocks {
    pub withdrawal_start: u64,   // When withdrawal period starts
    pub cancellation_start: u64, // When cancellation period starts
}

#[contract]
pub struct EscrowFactory;

#[contractimpl]
impl EscrowFactory {
    // Create a new escrow for atomic swap
    pub fn create_escrow(
        env: Env,
        immutables: EscrowImmutables,
    ) -> Address {
        // Deploy new escrow contract with deterministic address
        let salt = env.crypto().sha256(&immutables.to_xdr(&env));
        let wasm_hash = env.deployer().upload_contract_wasm(env.current_contract_wasm());
        let contract_address = env.deployer().with_current_contract(salt)
            .deploy_v2(wasm_hash, ());
        
        // Transfer tokens to escrow
        if immutables.token == Address::from_contract_id(&[0; 32].into()) {
            // Handle native token transfer
            env.transfer_from_balance(immutables.maker.clone(), contract_address.clone(), immutables.amount);
        } else {
            // Handle ERC20-like token transfer
            env.invoke_contract::<()>(
                &immutables.token,
                &Symbol::new(&env, "transfer_from"),
                Vec::from_array(&env, [immutables.maker.into(), contract_address.into(), immutables.amount.into()]),
            );
        }
        
        // Initialize escrow contract
        env.invoke_contract::<()>(
            &contract_address,
            &Symbol::new(&env, "initialize"),
            Vec::from_array(&env, [immutables.into()]),
        );
        
        contract_address
    }
}
