use soroban_sdk::{contracttype, contracterror, Address, BytesN, Env};

use crate::libraries::timelocks_lib::{Timelocks};

// Events
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct EscrowCancelled;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct FundsRescued {
    pub token: Address,
    pub amount: i128,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Withdrawal {
    pub secret: BytesN<32>,
}

// Errors
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum BaseEscrowError {
    InvalidCaller = 1,
    InvalidImmutables = 2,
    InvalidSecret = 3,
    InvalidTime = 4,
    NativeTokenSendingFailure = 5,
}

// Structs
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Immutables {
    pub order_hash: BytesN<32>,
    pub hashlock: BytesN<32>,  // Hash of the secret
    pub maker: Address,
    pub taker: Address,
    pub token: Address,
    pub amount: i128,
    pub safety_deposit: i128,
    pub timelocks: Timelocks,
}

// Constants (would be implemented as associated functions or constants in the contract)
pub trait BaseEscrowTrait {
    /// Returns the delay for rescuing funds from the escrow (in seconds)
    fn rescue_delay(&self, env: &Env) -> u32;
    
    /// Returns the address of the factory that created the escrow
    fn factory(&self, env: &Env) -> Address;
    
    /// Withdraws funds to a predetermined recipient
    /// Withdrawal can only be made during the withdrawal period and with secret matching hashlock
    /// The safety deposit is sent to the caller
    fn withdraw(&self, env: &Env, secret: BytesN<32>, immutables: &Immutables) -> Result<(), BaseEscrowError>;
    
    /// Cancels the escrow and returns tokens to a predetermined recipient
    /// The escrow can only be cancelled during the cancellation period
    /// The safety deposit is sent to the caller
    fn cancel(&self, env: &Env, immutables: &Immutables) -> Result<(), BaseEscrowError>;
    
    /// Rescues funds from the escrow
    /// Funds can only be rescued by the taker after the rescue delay
    fn rescue_funds(
        &self,
        env: &Env,
        token: Address,
        amount: i128,
        immutables: &Immutables
    ) -> Result<(), BaseEscrowError>;
}

// Helper functions for time validation (semantic equivalent to Solidity's Timelocks.get())
// impl Immutables {
//     pub fn is_withdrawal_period(&self, env: &Env, is_source: bool) -> bool {
//         let stage = if is_source {
//             Stage::SrcWithdrawal
//         } else {
//             Stage::DstWithdrawal
//         };
//         let current_time = env.ledger().timestamp();
//         let withdrawal_start = self.timelocks.get(stage);
//         current_time >= withdrawal_start.into()
//     }
    
//     pub fn is_public_withdrawal_period(&self, env: &Env, is_source: bool) -> bool {
//         let stage = if is_source {
//             Stage::SrcPublicWithdrawal
//         } else {
//             Stage::DstPublicWithdrawal
//         };
//         let current_time = env.ledger().timestamp();
//         let public_withdrawal_start = self.timelocks.get(stage);
//         current_time >= public_withdrawal_start.into()
//     }
    
//     pub fn is_cancellation_period(&self, env: &Env, is_source: bool) -> bool {
//         let stage = if is_source {
//             Stage::SrcCancellation
//         } else {
//             Stage::DstCancellation
//         };
//         let current_time = env.ledger().timestamp();
//         let cancellation_start = self.timelocks.get(stage);
//         current_time >= cancellation_start.into()
//     }
    
//     pub fn is_public_cancellation_period(&self, env: &Env) -> bool {
//         let current_time = env.ledger().timestamp();
//         let public_cancellation_start = self.timelocks.get(Stage::SrcPublicCancellation);
//         current_time >= public_cancellation_start.into()
//     }
    
//     pub fn is_rescue_available(&self, env: &Env, rescue_delay: u32) -> bool {
//         let current_time = env.ledger().timestamp();
//         let rescue_start = self.timelocks.rescue_start(rescue_delay);
//         current_time >= rescue_start.into()
//     }
    
//     pub fn verify_secret(&self, secret: &BytesN<32>) -> bool {
//         let computed_hashlock = soroban_sdk::crypto::sha256(&secret.clone().into());
//         computed_hashlock == self.hashlock
//     }
// }