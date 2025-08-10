use anchor_lang::prelude::*;

// Main program state managing the token exchange system
#[account]
pub struct Redeem {
    // Authority that can manage the system
    pub authority: Pubkey,
    // Mint address for the ticket tokens
    pub ticket_mint: Pubkey,
    // Vault to collect SOL payments
    pub sol_vault: Pubkey,
    // SOL lamports per ticket token
    pub sol_per_ticket: u64,
    // Total tickets minted
    pub total_tickets_minted: u64,
    // Total tickets redeemed
    pub total_tickets_redeemed: u64,
    // System is active
    pub is_active: bool,
    // Bump seed for PDA
    pub bump: u8,
}

impl Redeem {
    pub const LEN: usize = 8 + // discriminator
        32 + // authority
        32 + // ticket_mint
        32 + // sol_vault
        8 +  // sol_per_ticket
        8 +  // total_tickets_minted
        8 +  // total_tickets_redeemed
        1 +  // is_active
        1;   // bump

    pub fn calculate_sol_cost(&self, ticket_amount: u64) -> Result<u64> {
        self.sol_per_ticket
            .checked_mul(ticket_amount)
            .ok_or(ErrorCode::MathOverflow.into())
    }
}

// Product available for redemption
#[account]
pub struct Product {
    // Product ID (unique identifier)
    pub id: u64,
    // Product name (32 bytes max)
    pub name: String,
    // Product description (64 bytes max)
    pub description: String,
    // Ticket cost to redeem this product
    pub ticket_cost: u64,
    // Total quantity available
    pub total_quantity: u32,
    // Quantity already redeemed
    pub redeemed_quantity: u32,
    // Product is active and available
    pub is_active: bool,
    // Authority that created this product
    pub authority: Pubkey,
    // Bump seed for PDA
    pub bump: u8,
}

impl Product {
    pub const LEN: usize = 8 +
        8 + // id
        32 + // name
        64 + // description
        8 +  // ticket_cost
        4 +  // total_quantity
        4 +  // redeemed_quantity
        1 +  // is_active
        32 + // authority
        1;   // bump

    pub fn is_available(&self) -> bool {
        self.is_active && self.redeemed_quantity < self.total_quantity
    }

    pub fn remaining_quantity(&self) -> u32 {
        self.total_quantity.saturating_sub(self.redeemed_quantity)
    }
}
