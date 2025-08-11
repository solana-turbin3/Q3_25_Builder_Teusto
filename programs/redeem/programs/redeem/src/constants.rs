use anchor_lang::prelude::*;

/// PDA SEEDS - These are the deterministic seeds used to derive Program Derived Addresses (PDAs)
/// PDAs are accounts owned by the program that can be derived deterministically from seeds

/// Main system state PDA seed
/// This creates a single, global account that holds the system configuration
pub const REDEEM_SEED: &[u8] = b"redeem";

/// SOL vault PDA seed - stores all collected SOL payments
/// Combined with the redeem account address to ensure uniqueness
pub const SOL_VAULT_SEED: &[u8] = b"sol_vault";

/// Product catalog PDA seed - each product gets its own account
/// Combined with product_id to create unique addresses for each product
pub const PRODUCT_SEED: &[u8] = b"product";

/// User ticket account PDA seed - tracks each user's ticket balance and history
/// Combined with user's public key to create unique addresses per user
pub const USER_REDEEM_SEED: &[u8] = b"user_redeem";

/// Redemption record PDA seed - creates audit trail for each redemption
/// Combined with user, product_id, and timestamp for unique records
pub const REDEMPTION_SEED: &[u8] = b"redemption";

/// SYSTEM CONSTRAINTS - These define the operational limits of the program

/// Minimum SOL per ticket rate (0.001 SOL = 1,000,000 lamports)
/// Prevents setting exchange rates too low which could cause economic issues
pub const MIN_SOL_PER_TICKET: u64 = 1_000_000;

/// Maximum SOL per ticket rate (1 SOL = 1,000,000,000 lamports)
/// Prevents setting exchange rates too high which could price out users
pub const MAX_SOL_PER_TICKET: u64 = 1_000_000_000;

/// Minimum tickets that can be purchased in a single transaction
/// Prevents spam transactions and ensures meaningful purchases
pub const MIN_TICKET_PURCHASE: u64 = 1;

/// Maximum tickets that can be purchased in a single transaction
/// Prevents large purchases that could drain the system or cause overflow
pub const MAX_TICKET_PURCHASE: u64 = 1_000;

/// Minimum ticket cost for a product
/// Ensures products have meaningful value in the token economy
pub const MIN_PRODUCT_TICKET_COST: u64 = 1;

/// Maximum ticket cost for a product
/// Prevents products from being priced too high
pub const MAX_PRODUCT_TICKET_COST: u64 = 10_000;

/// Maximum product quantity that can be added
/// Prevents inventory overflow and ensures reasonable stock levels
pub const MAX_PRODUCT_QUANTITY: u32 = 10_000;

/// Maximum length for product names (in bytes)
/// Ensures product names fit within account size constraints
pub const MAX_PRODUCT_NAME_LEN: usize = 32;

/// Maximum length for product descriptions (in bytes)
/// Ensures descriptions fit within account size constraints
pub const MAX_PRODUCT_DESCRIPTION_LEN: usize = 64;

/// VALIDATION FUNCTIONS - These provide reusable validation logic

/// Validates that a SOL per ticket rate is within acceptable bounds
/// 
/// # Arguments
/// * `sol_per_ticket` - The rate to validate in lamports
/// 
/// # Returns
/// * `bool` - true if the rate is valid, false otherwise
pub fn is_valid_sol_per_ticket(sol_per_ticket: u64) -> bool {
    sol_per_ticket >= MIN_SOL_PER_TICKET && sol_per_ticket <= MAX_SOL_PER_TICKET
}

/// Validates that a ticket purchase amount is within acceptable bounds
/// 
/// # Arguments
/// * `amount` - The number of tickets to validate
/// 
/// # Returns
/// * `bool` - true if the amount is valid, false otherwise
pub fn is_valid_ticket_amount(amount: u64) -> bool {
    amount >= MIN_TICKET_PURCHASE && amount <= MAX_TICKET_PURCHASE
}

/// Validates that a product configuration is acceptable
/// 
/// # Arguments
/// * `ticket_cost` - The ticket cost for the product
/// * `quantity` - The total quantity of the product
/// * `name` - The product name
/// * `description` - The product description
/// 
/// # Returns
/// * `bool` - true if all parameters are valid, false otherwise
pub fn is_valid_product(
    ticket_cost: u64,
    quantity: u32,
    name: &str,
    description: &str,
) -> bool {
    ticket_cost >= MIN_PRODUCT_TICKET_COST
        && ticket_cost <= MAX_PRODUCT_TICKET_COST
        && quantity > 0
        && quantity <= MAX_PRODUCT_QUANTITY
        && !name.is_empty()
        && name.len() <= MAX_PRODUCT_NAME_LEN
        && description.len() <= MAX_PRODUCT_DESCRIPTION_LEN
}

/// UTILITY FUNCTIONS - Helper functions for common operations

/// Converts a string to a fixed-size byte array for storage
/// Pads with zeros if the string is shorter than the target size
/// 
/// # Arguments
/// * `input` - The string to convert
/// * `size` - The target size in bytes
/// 
/// # Returns
/// * `Vec<u8>` - The padded byte array
pub fn string_to_bytes(input: &str, size: usize) -> Vec<u8> {
    let mut bytes = input.as_bytes().to_vec();
    bytes.resize(size, 0);
    bytes
}

/// Converts a byte array back to a string, removing null padding
/// 
/// # Arguments
/// * `bytes` - The byte array to convert
/// 
/// # Returns
/// * `String` - The converted string with padding removed
pub fn bytes_to_string(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes)
        .trim_end_matches('\0')
        .to_string()
}

/// Calculates the total SOL cost for a given number of tickets
/// Uses checked arithmetic to prevent overflow
/// 
/// # Arguments
/// * `ticket_amount` - Number of tickets
/// * `sol_per_ticket` - Rate in lamports per ticket
/// 
/// # Returns
/// * `Option<u64>` - The total cost in lamports, or None if overflow
pub fn calculate_total_cost(ticket_amount: u64, sol_per_ticket: u64) -> Option<u64> {
    ticket_amount.checked_mul(sol_per_ticket)
}

/// Checks if a user has sufficient tickets for a redemption
/// 
/// # Arguments
/// * `user_balance` - User's current ticket balance
/// * `required_tickets` - Tickets needed for the redemption
/// 
/// # Returns
/// * `bool` - true if the user has enough tickets, false otherwise
pub fn has_sufficient_tickets(user_balance: u64, required_tickets: u64) -> bool {
    user_balance >= required_tickets
}

/// Generates a unique seed for redemption records
/// Combines user, product, and timestamp to ensure uniqueness
/// 
/// # Arguments
/// * `user` - User's public key
/// * `product_id` - Product identifier
/// * `timestamp` - Current timestamp
/// 
/// # Returns
/// * `Vec<Vec<u8>>` - Array of seeds for PDA derivation
pub fn redemption_seeds(user: &Pubkey, product_id: u64, timestamp: i64) -> Vec<Vec<u8>> {
    vec![
        REDEMPTION_SEED.to_vec(),
        user.to_bytes().to_vec(),
        product_id.to_le_bytes().to_vec(),
        timestamp.to_le_bytes().to_vec(),
    ]
}