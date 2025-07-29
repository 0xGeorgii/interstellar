use soroban_sdk::{contracttype, Address, Env};

/// Represents maker preferences for an order in a structured way
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq, Default)]
pub struct MakerTraits {
    // Flags (high bits in original implementation)
    pub no_partial_fills: bool,           // Equivalent to bit 255
    pub allow_multiple_fills: bool,       // Equivalent to bit 254
    pub pre_interaction_call: bool,       // Equivalent to bit 252
    pub post_interaction_call: bool,      // Equivalent to bit 251
    pub need_check_epoch_manager: bool,   // Equivalent to bit 250
    pub has_extension: bool,              // Equivalent to bit 249
    pub use_permit2: bool,                // Equivalent to bit 248
    pub unwrap_weth: bool,                // Equivalent to bit 247
    
    // Low bits data (originally in low 200 bits)
    pub allowed_sender: Option<Address>,  // None if any sender allowed, Some(address) if restricted
    pub expiration: Option<u64>,          // None if no expiration, Some(timestamp) if has expiration
    pub nonce_or_epoch: u64,              // Nonce or epoch value
    pub series: u64,                      // Series value
}

impl MakerTraits {
    /// Creates a new MakerTraits with default values
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Sets whether partial fills are allowed
    pub fn set_allow_partial_fills(&mut self, allow: bool) {
        self.no_partial_fills = !allow;
    }
    
    /// Sets whether multiple fills are allowed
    pub fn set_allow_multiple_fills(&mut self, allow: bool) {
        self.allow_multiple_fills = allow;
    }
    
    /// Sets pre-interaction call requirement
    pub fn set_pre_interaction_call(&mut self, need: bool) {
        self.pre_interaction_call = need;
    }
    
    /// Sets post-interaction call requirement
    pub fn set_post_interaction_call(&mut self, need: bool) {
        self.post_interaction_call = need;
    }
    
    /// Sets epoch manager check requirement
    pub fn set_need_check_epoch_manager(&mut self, need: bool) {
        self.need_check_epoch_manager = need;
    }
    
    /// Sets extension flag
    pub fn set_has_extension(&mut self, has: bool) {
        self.has_extension = has;
    }
    
    /// Sets permit2 usage
    pub fn set_use_permit2(&mut self, use_permit: bool) {
        self.use_permit2 = use_permit;
    }
    
    /// Sets WETH unwrapping requirement
    pub fn set_unwrap_weth(&mut self, unwrap: bool) {
        self.unwrap_weth = unwrap;
    }
    
    /// Sets allowed sender
    pub fn set_allowed_sender(&mut self, sender: Option<Address>) {
        self.allowed_sender = sender;
    }
    
    /// Sets expiration timestamp
    pub fn set_expiration(&mut self, expiration: Option<u64>) {
        self.expiration = expiration;
    }
    
    /// Sets nonce or epoch
    pub fn set_nonce_or_epoch(&mut self, nonce_or_epoch: u64) {
        self.nonce_or_epoch = nonce_or_epoch;
    }
    
    /// Sets series
    pub fn set_series(&mut self, series: u64) {
        self.series = series;
    }
}

/// Library functions for working with MakerTraits
pub struct MakerTraitsLib;

impl MakerTraitsLib {
    /**
     * @notice Checks if the order has the extension flag set.
     * @dev If the `has_extension` is true, then the protocol expects that the order has extension(s).
     * @param maker_traits The traits of the maker.
     * @return result A boolean indicating whether the flag is set.
     */
    pub fn has_extension(maker_traits: &MakerTraits) -> bool {
        maker_traits.has_extension
    }

    /**
     * @notice Checks if the maker allows a specific taker to fill the order.
     * @param maker_traits The traits of the maker.
     * @param sender The address of the taker to be checked.
     * @return result A boolean indicating whether the taker is allowed.
     */
    pub fn is_allowed_sender(maker_traits: &MakerTraits, sender: &Address) -> bool {
        match &maker_traits.allowed_sender {
            None => true, // Any sender allowed
            Some(allowed) => allowed == sender,
        }
    }

    /**
     * @notice Checks if the order has expired.
     * @param maker_traits The traits of the maker.
     * @param env Environment to get current timestamp
     * @return result A boolean indicating whether the order has expired.
     */
    pub fn is_expired(maker_traits: &MakerTraits, env: &Env) -> bool {
        match maker_traits.expiration {
            None => false, // No expiration
            Some(expiration) => {
                let current_time = env.ledger().timestamp();
                expiration < current_time
            }
        }
    }

    /**
     * @notice Returns the nonce or epoch of the order.
     * @param maker_traits The traits of the maker.
     * @return result The nonce or epoch of the order.
     */
    pub fn nonce_or_epoch(maker_traits: &MakerTraits) -> u64 {
        maker_traits.nonce_or_epoch
    }

    /**
     * @notice Returns the series of the order.
     * @param maker_traits The traits of the maker.
     * @return result The series of the order.
     */
    pub fn series(maker_traits: &MakerTraits) -> u64 {
        maker_traits.series
    }

    /**
     * @notice Determines if the order allows partial fills.
     * @dev If the no_partial_fills is false, then the order allows partial fills.
     * @param maker_traits The traits of the maker, determining their preferences for the order.
     * @return result A boolean indicating whether the maker allows partial fills.
     */
    pub fn allow_partial_fills(maker_traits: &MakerTraits) -> bool {
        !maker_traits.no_partial_fills
    }

    /**
     * @notice Checks if the maker needs pre-interaction call.
     * @param maker_traits The traits of the maker.
     * @return result A boolean indicating whether the maker needs a pre-interaction call.
     */
    pub fn need_pre_interaction_call(maker_traits: &MakerTraits) -> bool {
        maker_traits.pre_interaction_call
    }

    /**
     * @notice Checks if the maker needs post-interaction call.
     * @param maker_traits The traits of the maker.
     * @return result A boolean indicating whether the maker needs a post-interaction call.
     */
    pub fn need_post_interaction_call(maker_traits: &MakerTraits) -> bool {
        maker_traits.post_interaction_call
    }

    /**
     * @notice Determines if the order allows multiple fills.
     * @dev If the allow_multiple_fills is true, then the maker allows multiple fills.
     * @param maker_traits The traits of the maker, determining their preferences for the order.
     * @return result A boolean indicating whether the maker allows multiple fills.
     */
    pub fn allow_multiple_fills(maker_traits: &MakerTraits) -> bool {
        maker_traits.allow_multiple_fills
    }

    /**
     * @notice Determines if an order should use the bit invalidator or remaining amount validator.
     * @dev The bit invalidator can be used if the order does not allow partial or multiple fills.
     * @param maker_traits The traits of the maker, determining their preferences for the order.
     * @return result A boolean indicating whether the bit invalidator should be used.
     * True if the order requires the use of the bit invalidator.
     */
    pub fn use_bit_invalidator(maker_traits: &MakerTraits) -> bool {
        !Self::allow_partial_fills(maker_traits) || !Self::allow_multiple_fills(maker_traits)
    }

    /**
     * @notice Checks if the maker needs to check the epoch.
     * @param maker_traits The traits of the maker.
     * @return result A boolean indicating whether the maker needs to check the epoch manager.
     */
    pub fn need_check_epoch_manager(maker_traits: &MakerTraits) -> bool {
        maker_traits.need_check_epoch_manager
    }

    /**
     * @notice Checks if the maker uses permit2.
     * @param maker_traits The traits of the maker.
     * @return result A boolean indicating whether the maker uses permit2.
     */
    pub fn use_permit2(maker_traits: &MakerTraits) -> bool {
        maker_traits.use_permit2
    }

    /**
     * @notice Checks if the maker needs to unwraps WETH.
     * @param maker_traits The traits of the maker.
     * @return result A boolean indicating whether the maker needs to unwrap WETH.
     */
    pub fn unwrap_weth(maker_traits: &MakerTraits) -> bool {
        maker_traits.unwrap_weth
    }
}