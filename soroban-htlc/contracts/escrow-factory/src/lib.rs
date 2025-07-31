// lib.rs
#![no_std]
use soroban_sdk::{
    contract, contracterror, contractimpl, contractmeta, contracttype,
    panic_with_error, token, xdr::ToXdr, Address, BytesN, Env, IntoVal, Symbol
};

contractmeta!(
    key = "Description",
    val = "Bare-bone cross-chain atomic swap escrow factory"
);

#[derive(Clone)]
#[contracttype]
pub struct EscrowImmutables {
    pub hashlock: BytesN<32>,  // Hash of the secret
    pub direction: EscrowDirection,
    pub maker: Address,
    // pub taker: Address,
    pub token: Address,
    pub amount: i128,
    pub safety_deposit_token: Address,
    pub safety_deposit_amount: i128,
    pub timelocks: TimeLocks,  // Timelocks for withdrawal and cancellation
}

#[derive(Clone)]
#[contracttype]
pub enum EscrowDirection {
    Maker2Taker,
    Taker2Maker,
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
pub struct EscrowFactory;

#[contractimpl]
impl EscrowFactory {
    // Create a new escrow for atomic swap
    pub fn create_escrow(
        env: Env,
        immutables: EscrowImmutables,
        taker: Address,
    ) -> Address {
        // Deploy new escrow contract with deterministic address
        let salt = env.crypto().sha256(&immutables.clone().to_xdr(&env));
        let address = env.deployer().with_current_contract(salt).deployed_address();
        let escrow_client = EscrowClient::new(&env, &address);
        
        // Transfer tokens to escrow
        let sender = match immutables.direction {
            EscrowDirection::Maker2Taker => {
                immutables.maker.require_auth_for_args((immutables.clone(),).into_val(&env));
                &immutables.maker
            },
            EscrowDirection::Taker2Maker => {
                taker.require_auth();
                &taker
            },
        };
        
        token::Client::new(&env, &immutables.token)
            .transfer(sender, &address, &immutables.amount);

        taker.require_auth();
        token::Client::new(&env, &immutables.safety_deposit_token)
            .transfer(&taker, &address, &immutables.safety_deposit_amount);
        
        // Initialize escrow contracts
        escrow_client.initialize(&immutables, &taker);
        
        address
    }
}

#[contract]
pub struct Escrow;

#[contractimpl]
impl Escrow {
    // Initialize escrow with immutables
    pub fn initialize(env: Env, immutables: EscrowImmutables, taker: Address) {
        env.storage().instance().set(&Symbol::new(&env, "state"), &EscrowState::Active);
        env.storage().instance().set(&Symbol::new(&env, "immutables"), &immutables);
        env.storage().instance().set(&Symbol::new(&env, "taker"), &taker);
    }
    
    // Withdraw funds with secret
    pub fn withdraw(env: Env, secret: BytesN<32>, caller: Address) {
        let immutables: EscrowImmutables = env.storage().instance()
            .get(&Symbol::new(&env, "immutables"))
            .unwrap();
        
        let state: EscrowState = env.storage().instance()
            .get(&Symbol::new(&env, "state"))
            .unwrap();
        
        let taker: Address = env.storage().instance()
            .get(&Symbol::new(&env, "taker"))
            .unwrap();

        let sender = env.current_contract_address();
        
        let payee = match immutables.direction {
            EscrowDirection::Maker2Taker => taker,
            EscrowDirection::Taker2Maker => immutables.maker,
        };
        
        // Validate state
        if !matches!(state, EscrowState::Active) {
            panic_with_error!(&env, EscrowError::NotActive);
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

        // Transfer tokens
        token::Client::new(&env, &immutables.token)
            .transfer(&sender, &payee, &immutables.amount);
        
        // Transfer safety deposit to caller
        token::Client::new(&env, &immutables.safety_deposit_token)
            .transfer(&sender, &caller, &immutables.safety_deposit_amount);
        
        // Update state
        env.storage().instance().set(&Symbol::new(&env, "state"), &EscrowState::Withdrawn);
        
        // Emit event
        env.events().publish(
            (Symbol::new(&env, "withdraw"),),
            (secret,)
        );
    }
    
    // Cancel escrow and return funds
    pub fn cancel(env: Env, caller: Address) {
        let immutables: EscrowImmutables = env.storage().instance()
            .get(&Symbol::new(&env, "immutables"))
            .unwrap();
        
        let state: EscrowState = env.storage().instance()
            .get(&Symbol::new(&env, "state"))
            .unwrap();
        
        let taker: Address = env.storage().instance()
            .get(&Symbol::new(&env, "taker"))
            .unwrap();

        let sender = env.current_contract_address();
        
        let payee = match immutables.direction {
            EscrowDirection::Maker2Taker => immutables.maker,
            EscrowDirection::Taker2Maker => taker,
        };
        
        // Validate state
        if !matches!(state, EscrowState::Active) {
            panic_with_error!(&env, EscrowError::NotActive);
        }
        
        // Validate time
        let current_time = env.ledger().timestamp();
        if current_time < immutables.timelocks.cancellation_start {
            panic_with_error!(&env, EscrowError::TooEarly);
        }

        // Transfer tokens back
        token::Client::new(&env, &immutables.token)
            .transfer(&sender, &payee, &immutables.amount);
        
        // Transfer safety deposit to caller
        token::Client::new(&env, &immutables.safety_deposit_token)
            .transfer(&sender, &caller, &immutables.safety_deposit_amount);
        
        // Update state
        env.storage().instance().set(&Symbol::new(&env, "state"), &EscrowState::Cancelled);
        
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
    
    // Get escrow taker
    pub fn get_taker(env: Env) -> Address {
        env.storage().instance()
            .get(&Symbol::new(&env, "taker"))
            .unwrap()
    }
}
