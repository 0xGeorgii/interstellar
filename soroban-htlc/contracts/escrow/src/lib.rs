// lib.rs
#![no_std]
use soroban_sdk::{
    contract, contractimpl, contractmeta, contracttype, contracterror,
    panic_with_error, Address, BytesN, Env, Symbol, token
};

contractmeta!(
    key = "Description",
    val = "Bare-bone cross-chain atomic swap escrow"
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

#[derive(Clone)]
#[contracttype]
pub enum EscrowState {
    Active,
    Withdrawn,
    Cancelled,
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum EscrowError {
    NotActive = 1,
    Unauthorized = 2,
    TooEarly = 3,
    TooLate = 4,
    InvalidSecret = 5,
}

#[contract]
pub struct Escrow;

#[contractimpl]
impl Escrow {
    // Initialize escrow with immutables
    pub fn initialize(env: Env, immutables: EscrowImmutables) {
        env.storage().instance().set(&Symbol::new(&env, "state"), &EscrowState::Active);
        env.storage().instance().set(&Symbol::new(&env, "immutables"), &immutables);
    }
    
    // Withdraw funds to taker with secret
    pub fn withdraw(env: Env, secret: BytesN<32>, caller: Address) {
        let immutables: EscrowImmutables = env.storage().instance()
            .get(&Symbol::new(&env, "immutables"))
            .unwrap();
        
        let mut state: EscrowState = env.storage().instance()
            .get(&Symbol::new(&env, "state"))
            .unwrap();
        
        // Validate state
        if !matches!(state, EscrowState::Active) {
            panic_with_error!(&env, EscrowError::NotActive);
        }
        
        // Validate caller
        if caller != immutables.taker {
            panic_with_error!(&env, EscrowError::Unauthorized);
        }
        
        // Validate time
        let current_time = env.ledger().timestamp();
        if current_time < immutables.timelocks.withdrawal_start {
            panic_with_error!(&env, EscrowError::TooEarly);
        }
        if current_time >= immutables.timelocks.cancellation_start {
            panic_with_error!(&env, EscrowError::TooLate);
        }
        
        // Validate secret
        let secret_hash = env.crypto().sha256(secret.as_ref());
        if secret_hash.to_bytes() != immutables.hashlock {
            panic_with_error!(&env, EscrowError::InvalidSecret);
        }

        let token_client = token::Client::new(&env, &immutables.token);

        // Transfer tokens to taker
        token_client.transfer(
            &env.current_contract_address(),
            &immutables.taker,
            &immutables.amount
        );
        
        // Transfer safety deposit to caller
        token_client.transfer(
            &env.current_contract_address(),
            &caller,
            &immutables.safety_deposit
        );
        
        // Update state
        state = EscrowState::Withdrawn;
        env.storage().instance().set(&Symbol::new(&env, "state"), &state);
        
        // Emit event
        env.events().publish(
            (Symbol::new(&env, "withdraw"),),
            (secret,)
        );
    }
    
    // Cancel escrow and return funds to maker
    pub fn cancel(env: Env, caller: Address) {
        let immutables: EscrowImmutables = env.storage().instance()
            .get(&Symbol::new(&env, "immutables"))
            .unwrap();
        
        let mut state: EscrowState = env.storage().instance()
            .get(&Symbol::new(&env, "state"))
            .unwrap();
        
        // Validate state
        if !matches!(state, EscrowState::Active) {
            panic_with_error!(&env, EscrowError::NotActive);
        }
        
        // Validate time
        let current_time = env.ledger().timestamp();
        if current_time < immutables.timelocks.cancellation_start {
            panic_with_error!(&env, EscrowError::TooEarly);
        }
        
        let token_client = token::Client::new(&env, &immutables.token);

        // Transfer tokens back to maker
        token_client.transfer(
            &env.current_contract_address(),
            &immutables.maker,
            &immutables.amount
        );
        
        // Transfer safety deposit to caller
        token_client.transfer(
            &env.current_contract_address(),
            &caller,
            &immutables.safety_deposit
        );
        
        // Update state
        state = EscrowState::Cancelled;
        env.storage().instance().set(&Symbol::new(&env, "state"), &state);
        
        // Emit event
        env.events().publish(
            (Symbol::new(&env, "cancel"),),
            ()
        );
    }
    
    // Get escrow immutables
    pub fn get_immutables(env: Env) -> EscrowImmutables {
        env.storage().instance()
            .get(&Symbol::new(&env, "immutables"))
            .unwrap()
    }
    
    // Get escrow state
    pub fn get_state(env: Env) -> EscrowState {
        env.storage().instance()
            .get(&Symbol::new(&env, "state"))
            .unwrap()
    }
}
