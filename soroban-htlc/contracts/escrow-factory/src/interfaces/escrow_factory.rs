use soroban_sdk::{
    contracttype, contracterror, Address, Bytes, BytesN, Env
};

use crate::{interfaces::base_escrow::{BaseEscrowError, Immutables}, libraries::Timelocks};

// Events
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SrcEscrowCreated {
    pub src_immutables: Immutables,
    pub dst_immutables_complement: DstImmutablesComplement,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct DstEscrowCreated {
    pub escrow: Address,
    pub hashlock: BytesN<32>,
    pub taker: Address,
}

// Errors
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum EscrowFactoryError {
    InsufficientEscrowBalance = 1,
    InvalidCreationTime = 2,
    InvalidPartialFill = 3,
    InvalidSecretsAmount = 4,
    BaseEscrowError = 5, // Wrapper for BaseEscrow errors
}

// Structs
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExtraDataArgs {
    pub hashlock_info: BytesN<32>, // Hash of the secret or the Merkle tree root if multiple fills are allowed
    pub dst_chain_id: u32,
    pub dst_token: Address,
    pub deposits: u128,
    pub timelocks: Timelocks,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DstImmutablesComplement {
    pub maker: Address,
    pub amount: i128,
    pub token: Address,
    pub safety_deposit: i128,
    pub chain_id: u32,
}

// Factory Interface Trait
pub trait EscrowFactoryTrait {
    /// Returns the address of implementation on the source chain
    fn escrow_src_implementation(&self, env: &Env) -> Address;
    
    /// Returns the address of implementation on the destination chain
    fn escrow_dst_implementation(&self, env: &Env) -> Address;
    
    /// Creates a new escrow contract for taker on the destination chain
    /// The caller must send the safety deposit in the native token along with the function call
    /// and approve the destination token to be transferred to the created escrow
    fn create_dst_escrow(
        &self,
        env: &Env,
        dst_immutables: &Immutables,
        src_cancellation_timestamp: u64, // Using u64 for timestamp in Soroban
    ) -> Result<(), EscrowFactoryError>;
    
    /// Returns the deterministic address of the source escrow based on the salt
    fn address_of_escrow_src(&self, env: &Env, immutables: &Immutables) -> Address;
    
    /// Returns the deterministic address of the destination escrow based on the salt
    fn address_of_escrow_dst(&self, env: &Env, immutables: &Immutables) -> Address;
}

// Helper functions for common operations
impl Immutables {
    /// Generate a deterministic salt for escrow address calculation
    pub fn generate_salt(&self, env: &Env) -> BytesN<32> {
        // Create a deterministic hash based on the immutable fields
        // For simplicity, we'll hash the order_hash which should be unique per escrow
        let order_hash_bytes: Bytes = self.order_hash.clone().into();
        env.crypto().sha256(&order_hash_bytes).into()
    }
}

// Conversion from factory error to base escrow error
impl From<BaseEscrowError> for EscrowFactoryError {
    fn from(_error: BaseEscrowError) -> Self {
        EscrowFactoryError::BaseEscrowError
    }
}
