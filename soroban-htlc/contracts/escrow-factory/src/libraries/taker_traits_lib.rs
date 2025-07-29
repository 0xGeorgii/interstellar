use soroban_sdk::{contracttype};

/// Represents taker preferences for an order in a structured way
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq, Default)]
pub struct TakerTraits {
    // Flags (high bits in original implementation)
    pub is_making_amount: bool,           // Equivalent to bit 255 (_MAKER_AMOUNT_FLAG)
    pub unwrap_weth: bool,                // Equivalent to bit 254 (_UNWRAP_WETH_FLAG)
    pub skip_maker_permit: bool,          // Equivalent to bit 253 (_SKIP_ORDER_PERMIT_FLAG)
    pub use_permit2: bool,                // Equivalent to bit 252 (_USE_PERMIT2_FLAG)
    pub args_has_target: bool,            // Equivalent to bit 251 (_ARGS_HAS_TARGET)
    
    // Calldata lengths (bits 224-247 and 200-223 in original)
    pub args_extension_length: u32,       // Equivalent to _ARGS_EXTENSION_LENGTH (24 bits)
    pub args_interaction_length: u32,     // Equivalent to _ARGS_INTERACTION_LENGTH (24 bits)
    
    // Threshold amount (low 185 bits in original)
    pub threshold: u128,                  // Equivalent to threshold amount
}

impl TakerTraits {
    /// Creates a new TakerTraits with default values
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Sets whether taking amount should be calculated based on making amount
    pub fn set_is_making_amount(&mut self, is_making: bool) {
        self.is_making_amount = is_making;
    }
    
    /// Sets WETH unwrapping requirement
    pub fn set_unwrap_weth(&mut self, unwrap: bool) {
        self.unwrap_weth = unwrap;
    }
    
    /// Sets maker permit skipping
    pub fn set_skip_maker_permit(&mut self, skip: bool) {
        self.skip_maker_permit = skip;
    }
    
    /// Sets permit2 usage
    pub fn set_use_permit2(&mut self, use_permit: bool) {
        self.use_permit2 = use_permit;
    }
    
    /// Sets whether args should contain target address
    pub fn set_args_has_target(&mut self, has_target: bool) {
        self.args_has_target = has_target;
    }
    
    /// Sets extension calldata length
    pub fn set_args_extension_length(&mut self, length: u32) {
        self.args_extension_length = length;
    }
    
    /// Sets interaction calldata length
    pub fn set_args_interaction_length(&mut self, length: u32) {
        self.args_interaction_length = length;
    }
    
    /// Sets threshold amount
    pub fn set_threshold(&mut self, threshold: u128) {
        self.threshold = threshold;
    }
}

/// Library functions for working with TakerTraits
pub struct TakerTraitsLib;

impl TakerTraitsLib {
    /**
     * @notice Checks if the args should contain target address.
     * @param taker_traits The traits of the taker.
     * @return result A boolean indicating whether the args should contain target address.
     */
    pub fn args_has_target(taker_traits: &TakerTraits) -> bool {
        taker_traits.args_has_target
    }

    /**
     * @notice Retrieves the length of the extension calldata from the taker_traits.
     * @param taker_traits The traits of the taker.
     * @return result The length of the extension calldata encoded in the taker_traits.
     */
    pub fn args_extension_length(taker_traits: &TakerTraits) -> u32 {
        taker_traits.args_extension_length
    }

    /**
     * @notice Retrieves the length of the interaction calldata from the taker_traits.
     * @param taker_traits The traits of the taker.
     * @return result The length of the interaction calldata encoded in the taker_traits.
     */
    pub fn args_interaction_length(taker_traits: &TakerTraits) -> u32 {
        taker_traits.args_interaction_length
    }

    /**
     * @notice Checks if the taking amount should be calculated based on making amount.
     * @param taker_traits The traits of the taker.
     * @return result A boolean indicating whether the taking amount should be calculated based on making amount.
     */
    pub fn is_making_amount(taker_traits: &TakerTraits) -> bool {
        taker_traits.is_making_amount
    }

    /**
     * @notice Checks if the order should unwrap WETH and send ETH to taker.
     * @param taker_traits The traits of the taker.
     * @return result A boolean indicating whether the order should unwrap WETH.
     */
    pub fn unwrap_weth(taker_traits: &TakerTraits) -> bool {
        taker_traits.unwrap_weth
    }

    /**
     * @notice Checks if the order should skip maker's permit execution.
     * @param taker_traits The traits of the taker.
     * @return result A boolean indicating whether the order don't apply permit.
     */
    pub fn skip_maker_permit(taker_traits: &TakerTraits) -> bool {
        taker_traits.skip_maker_permit
    }

    /**
     * @notice Checks if the order uses the permit2 instead of permit.
     * @param taker_traits The traits of the taker.
     * @return result A boolean indicating whether the order uses the permit2.
     */
    pub fn use_permit2(taker_traits: &TakerTraits) -> bool {
        taker_traits.use_permit2
    }

    /**
     * @notice Retrieves the threshold amount from the taker_traits.
     * The maximum amount a taker agrees to give in exchange for a making amount.
     * @param taker_traits The traits of the taker.
     * @return result The threshold amount encoded in the taker_traits.
     */
    pub fn threshold(taker_traits: &TakerTraits) -> u128 {
        taker_traits.threshold
    }
}