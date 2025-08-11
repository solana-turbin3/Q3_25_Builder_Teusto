use anchor_lang::prelude::*;

// Program modules
pub mod state;
pub mod constants;
pub mod instructions;

// Re-export for external use
pub use state::*;
pub use constants::*;
pub use instructions::*;

// Program ID - This will be replaced with actual deployed program ID
declare_id!("6GzrsBjRd5N3cMfMhoZMvEr5STWXRV8wJEYGLsAw4vAA");

/// Redeem Program - Ticket Token Exchange System
/// 
/// This program enables users to:
/// 1. Purchase ticket tokens with SOL
/// 2. Redeem ticket tokens for real products
/// 3. Manage product catalogs (admin only)
/// 4. Track all transactions with audit trails
/// 
/// The program implements a complete token economy with:
/// - Configurable exchange rates
/// - Product inventory management
/// - User balance tracking
/// - Comprehensive security and validation
/// - Event emission for off-chain integration
#[program]
pub mod redeem {
    use super::*;

    /// Initialize the ticket exchange system
    /// 
    /// Creates the main system state, ticket mint, and SOL vault.
    /// Sets the exchange rate and activates the system.
    /// 
    /// # Arguments
    /// * `ctx` - Instruction context with required accounts
    /// * `sol_per_ticket` - Exchange rate in lamports per ticket
    /// 
    /// # Access Control
    /// Only the authority can call this instruction
    pub fn initialize(ctx: Context<Initialize>, sol_per_ticket: u64) -> Result<()> {
        instructions::initialize::handler(ctx, sol_per_ticket)
    }

    /// Purchase ticket tokens with SOL
    /// 
    /// Allows users to invest SOL and receive ticket tokens.
    /// Creates user accounts automatically on first purchase.
    /// 
    /// # Arguments
    /// * `ctx` - Instruction context with required accounts
    /// * `ticket_amount` - Number of tickets to purchase
    /// 
    /// # Access Control
    /// Any user can call this instruction
    pub fn purchase_tickets(ctx: Context<PurchaseTickets>, ticket_amount: u64) -> Result<()> {
        instructions::purchase_tickets::handler(ctx, ticket_amount)
    }

    /// Add a new product to the catalog
    /// 
    /// Creates a new product that users can redeem with tickets.
    /// Sets product configuration including cost and inventory.
    /// 
    /// # Arguments
    /// * `ctx` - Instruction context with required accounts
    /// * `product_id` - Unique identifier for the product
    /// * `name` - Product name (max 32 bytes)
    /// * `description` - Product description (max 64 bytes)
    /// * `ticket_cost` - Tickets required to redeem this product
    /// * `total_quantity` - Total inventory available
    /// 
    /// # Access Control
    /// Only the system authority can call this instruction
    pub fn add_product(
        ctx: Context<AddProduct>,
        product_id: u64,
        name: String,
        description: String,
        ticket_cost: u64,
        total_quantity: u32,
    ) -> Result<()> {
        instructions::add_product::handler(ctx, product_id, name, description, ticket_cost, total_quantity)
    }

    /// Redeem ticket tokens for a product
    /// 
    /// Burns user's ticket tokens and updates product inventory.
    /// Creates redemption record for audit trail.
    /// 
    /// # Arguments
    /// * `ctx` - Instruction context with required accounts
    /// * `product_id` - ID of the product to redeem
    /// 
    /// # Access Control
    /// Any user with sufficient tickets can call this instruction
    pub fn redeem_product(ctx: Context<RedeemProduct>, product_id: u64) -> Result<()> {
        instructions::redeem_product::handler(ctx, product_id)
    }
}
