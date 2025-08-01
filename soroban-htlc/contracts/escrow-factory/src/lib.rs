// lib.rs
#![no_std]
use soroban_sdk::{
    contract, contracterror, contractimpl, contractmeta, contracttype,
    panic_with_error, token, Address, Bytes, BytesN, Env, IntoVal, Symbol
};

contractmeta!(
    key = "Description",
    val = "Bare-bone cross-chain atomic swap escrow factory"
);

#[derive(Clone, PartialEq, Debug)]
#[contracttype]
pub struct EscrowImmutables {
    pub hashlock: BytesN<32>,  // Hash of the secret
    pub direction: EscrowDirection,
    pub maker: Address,
    pub token: Address,
    pub amount: AmountCalc,
    pub safety_deposit_token: Address,
    pub safety_deposit_amount: i128,
    pub timelocks: TimeLocks,  // Timelocks for withdrawal and cancellation
}

#[derive(Clone, PartialEq, Debug)]
#[contracttype]
pub enum EscrowDirection {
    Maker2Taker,
    Taker2Maker,
}

#[derive(Clone, PartialEq, Debug)]
#[contracttype]
pub enum AmountCalc {
    Flat(i128),
    Linear(DutchAuction),
}

impl AmountCalc {
    pub fn calc(&self, timestamp: u64) -> i128 {
        match self {
            AmountCalc::Flat(amount) => *amount,
            AmountCalc::Linear(da) => {
                let ts = timestamp.clamp(da.start_time, da.end_time);
                let a = da.start_amount * (da.end_time - ts) as i128;
                let b = da.end_amount * (ts - da.start_time) as i128;
                (a + b) / (da.end_time - da.start_time) as i128
            },
        }
    }
}

#[derive(Clone, PartialEq, Debug)]
#[contracttype]
pub struct DutchAuction {
    pub start_time: u64,
    pub end_time: u64,
    pub start_amount: i128,
    pub end_amount: i128,
}

#[derive(Clone, PartialEq, Debug)]
#[contracttype]
pub struct TimeLocks {
    pub withdrawal: u64,
    pub public_withdrawal: u64,
    pub cancellation: u64,
    pub public_cancellation: u64,
}

#[derive(Clone)]
#[contracttype]
pub struct EscrowResolves {
    taker: Address,
    amount: i128,
    timestamp: u64,
}

#[derive(Clone, PartialEq, Debug)]
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
    AlreadyTaken = 1,
    NotActive = 2,
    Unauthorized = 3,
    TooEarly = 4,
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
        let salt = immutables.hashlock.clone();
        let address = env.deployer().with_current_contract(salt).deployed_address();

        taker.require_auth();
        
        // Transfer tokens to escrow
        let sender = match immutables.direction {
            EscrowDirection::Maker2Taker => {
                immutables.maker.require_auth_for_args((immutables.clone(),).into_val(&env));
                &immutables.maker
            },
            EscrowDirection::Taker2Maker => &taker,
        };
        
        let timestamp = env.ledger().timestamp();
        
        let amount = immutables.amount.calc(timestamp);
        
        token::Client::new(&env, &immutables.token)
            .transfer(sender, &address, &amount);
        
        token::Client::new(&env, &immutables.safety_deposit_token)
            .transfer(&taker, &address, &immutables.safety_deposit_amount);
        
        // Initialize escrow contracts
        env.register_at(&address, Escrow, ());
        EscrowClient::new(&env, &address)
            .initialize(&immutables, &EscrowResolves { taker, amount, timestamp });
        
        address
    }
}

#[contract]
pub struct Escrow;

#[contractimpl]
impl Escrow {
    // Initialize escrow with immutables
    pub fn initialize(env: Env, immutables: EscrowImmutables, resolves: EscrowResolves) {
        if env.storage().instance().has(&Symbol::new(&env, "state")) {
            panic_with_error!(&env, EscrowError::AlreadyTaken);
        }
        
        env.storage().instance().set(&Symbol::new(&env, "state"), &EscrowState::Active);
        env.storage().instance().set(&Symbol::new(&env, "immutables"), &immutables);
        env.storage().instance().set(&Symbol::new(&env, "resolves"), &resolves);
    }
    
    // Withdraw funds with secret
    pub fn withdraw(env: Env, secret: Bytes, caller: Address) {
        let immutables: EscrowImmutables = env.storage().instance()
            .get(&Symbol::new(&env, "immutables"))
            .unwrap();
        
        let resolves: EscrowResolves = env.storage().instance()
            .get(&Symbol::new(&env, "resolves"))
            .unwrap();
        
        let state: EscrowState = env.storage().instance()
            .get(&Symbol::new(&env, "state"))
            .unwrap();
        
        let sender = env.current_contract_address();
        
        let payee = match immutables.direction {
            EscrowDirection::Maker2Taker => &resolves.taker,
            EscrowDirection::Taker2Maker => &immutables.maker,
        };
        
        // Validate state
        if !matches!(state, EscrowState::Active) {
            panic_with_error!(&env, EscrowError::NotActive);
        }
        
        // Validate time
        let start = resolves.timestamp + 
            if caller == resolves.taker {immutables.timelocks.withdrawal}
            else {immutables.timelocks.public_withdrawal};
        if env.ledger().timestamp() < start {
            panic_with_error!(&env, EscrowError::TooEarly);
        }
        
        // Validate secret
        let secret_hash = env.crypto().sha256(&secret);
        if secret_hash.to_bytes() != immutables.hashlock {
            panic_with_error!(&env, EscrowError::InvalidSecret);
        }
        
        // Transfer tokens
        token::Client::new(&env, &immutables.token)
            .transfer(&sender, &payee, &resolves.amount);
        
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
        
        let resolves: EscrowResolves = env.storage().instance()
            .get(&Symbol::new(&env, "resolves"))
            .unwrap();
        
        let state: EscrowState = env.storage().instance()
            .get(&Symbol::new(&env, "state"))
            .unwrap();
        
        let sender = env.current_contract_address();
        
        let payee = match immutables.direction {
            EscrowDirection::Maker2Taker => &immutables.maker,
            EscrowDirection::Taker2Maker => &resolves.taker,
        };
        
        // Validate state
        if !matches!(state, EscrowState::Active) {
            panic_with_error!(&env, EscrowError::NotActive);
        }
        
        // Validate time
        let start = resolves.timestamp + 
            if caller == resolves.taker {immutables.timelocks.cancellation}
            else {immutables.timelocks.public_cancellation};
        if env.ledger().timestamp() < start {
            panic_with_error!(&env, EscrowError::TooEarly);
        }
        
        // Require caller's auth
        caller.require_auth();
        
        // Transfer tokens back
        token::Client::new(&env, &immutables.token)
            .transfer(&sender, &payee, &resolves.amount);
        
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
    
    // Get escrow resolves
    pub fn get_resolves(env: Env) -> EscrowResolves {
        env.storage().instance()
            .get(&Symbol::new(&env, "resolves"))
            .unwrap()
    }
    
    // Get escrow state
    pub fn get_state(env: Env) -> EscrowState {
        env.storage().instance()
            .get(&Symbol::new(&env, "state"))
            .unwrap()
    }
}

mod test;