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

#[account]
pub struct UserRedeemAccount {
    // User's public key
    pub user: Pubkey,
    // Current ticket balance
    pub ticket_balance: u64,
    // Total tickets ever purchased
    pub total_purchased: u64,
    // Total tickets ever redeemed
    pub total_redeemed: u64,
    // Number of products redeemed
    pub products_redeemed: u32,
    // Account creation timestamp
    pub created_at: i64,
    // Last activity timestamp
    pub last_activity: i64,
    // Account is active
    pub is_active: bool,
    // Bump seed for PDA
    pub bump: u8,
}

impl UserRedeemAccount {
    pub const LEN: usize = 8 + // discriminator
        32 + // user
        8 +  // ticket_balance
        8 +  // total_purchased
        8 +  // total_redeemed
        4 +  // products_redeemed
        8 +  // created_at
        8 +  // last_activity
        1 +  // is_active
        1;   // bump

    pub fn can_redeem(&self, ticket_cost: u64) -> bool {
        self.is_active && self.ticket_balance >= ticket_cost
    }

    pub fn redeem_tickets(&mut self, amount: u64) -> Result<()> {
        require!(self.ticket_balance >= amount, ErrorCode::InsufficientTickets);
        
        self.ticket_balance = self.ticket_balance.saturating_sub(amount);
        self.total_redeemed = self.total_redeemed.saturating_add(amount);
        self.products_redeemed = self.products_redeemed.saturating_add(1);
        self.last_activity = Clock::get()?.unix_timestamp;
        
        Ok(())
    }

    pub fn add_tickets(&mut self, amount: u64) -> Result<()> {
        self.ticket_balance = self.ticket_balance
            .checked_add(amount)
            .ok_or(ErrorCode::MathOverflow)?;
        self.total_purchased = self.total_purchased
            .checked_add(amount)
            .ok_or(ErrorCode::MathOverflow)?;
        self.last_activity = Clock::get()?.unix_timestamp;
        
        Ok(())
    }
}

#[account]
pub struct RedemptionRecord {
    // User who made the redemption
    pub user: Pubkey,
    // Product that was redeemed
    pub product_id: u64,
    // Number of tickets used
    pub tickets_used: u64,
    // Timestamp of redemption
    pub redeemed_at: i64,
    // Transaction signature (for reference)
    pub transaction_signature: [u8; 64],
    // Redemption is valid and processed
    pub is_processed: bool,
    // Bump seed for PDA
    pub bump: u8,
}

impl RedemptionRecord {
    pub const LEN: usize = 8 + // discriminator
        32 + // user
        8 +  // product_id
        8 +  // tickets_used
        8 +  // redeemed_at
        64 + // transaction_signature
        1 +  // is_processed
        1;   // bump
}

#[error_code]
pub enum ErrorCode {
    #[msg("Math operation resulted in overflow")]
    MathOverflow,
    #[msg("Insufficient tickets for redemption")]
    InsufficientTickets,
    #[msg("Product is not available")]
    ProductNotAvailable,
    #[msg("Product is out of stock")]
    ProductOutOfStock,
    #[msg("Invalid ticket amount")]
    InvalidTicketAmount,
    #[msg("System is not active")]
    SystemNotActive,
    #[msg("Unauthorized access")]
    Unauthorized,
    #[msg("Invalid product configuration")]
    InvalidProduct,
    #[msg("User account not found")]
    UserAccountNotFound,
}
