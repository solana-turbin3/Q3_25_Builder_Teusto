use anchor_lang::prelude::*;
use crate::state::*;
use crate::constants::*;

/// Add a new product to the catalog
/// 
/// This instruction allows the system authority to add products that users can redeem:
/// 1. Validates product parameters (cost, quantity, name, description)
/// 2. Creates a new Product account with unique PDA
/// 3. Sets product configuration and availability
/// 4. Links product to the system authority
/// 
/// Only the system authority can call this instruction.
#[derive(Accounts)]
#[instruction(product_id: u64)]
pub struct AddProduct<'info> {
    /// System authority (must match redeem.authority)
    /// Only this account can add products to the catalog
    #[account(mut)]
    pub authority: Signer<'info>,

    /// Main system state (PDA)
    /// Used to verify authority and ensure system is active
    /// 
    /// Seeds: ["redeem"]
    /// Constraint: Authority must match and system must be active
    #[account(
        seeds = [REDEEM_SEED],
        bump = redeem.bump,
        constraint = redeem.authority == authority.key() @ ErrorCode::Unauthorized,
        constraint = redeem.is_active @ ErrorCode::SystemNotActive
    )]
    pub redeem: Account<'info, Redeem>,

    /// Product account (PDA) - stores product information
    /// Each product gets a unique account based on product_id
    /// 
    /// Seeds: ["product", product_id]
    /// Space: Product::LEN
    /// Payer: authority (pays for account creation)
    #[account(
        init,
        payer = authority,
        space = 8 + Product::LEN,
        seeds = [PRODUCT_SEED, product_id.to_le_bytes().as_ref()],
        bump
    )]
    pub product: Account<'info, Product>,

    /// Required system program
    pub system_program: Program<'info, System>,
}

/// Add product instruction handler
/// 
/// # Arguments
/// * `ctx` - The instruction context containing all accounts
/// * `product_id` - Unique identifier for the product
/// * `name` - Product name (max 32 bytes)
/// * `description` - Product description (max 64 bytes)
/// * `ticket_cost` - Number of tickets required to redeem this product
/// * `total_quantity` - Total inventory available for redemption
/// 
/// # Security Checks
/// 1. Validates caller is the system authority
/// 2. Ensures system is active
/// 3. Validates all product parameters are within bounds
/// 4. Ensures product_id is unique (handled by PDA init)
/// 
/// # State Changes
/// 1. Creates new Product account with provided configuration
/// 2. Sets product as active and available
/// 3. Links product to the authority that created it
pub fn handler(
    ctx: Context<AddProduct>,
    product_id: u64,
    name: String,
    description: String,
    ticket_cost: u64,
    total_quantity: u32,
) -> Result<()> {
    msg!("📦 Adding new product to catalog");
    msg!("   Product ID: {}", product_id);
    msg!("   Name: {}", name);
    msg!("   Description: {}", description);
    msg!("   Ticket Cost: {}", ticket_cost);
    msg!("   Total Quantity: {}", total_quantity);
    
    // Validate product parameters using our utility function
    require!(
        is_valid_product(ticket_cost, total_quantity, &name, &description),
        ErrorCode::InvalidProduct
    );
    
    // Additional validation for product ID (must be non-zero)
    require!(product_id > 0, ErrorCode::InvalidProduct);
    
    // Get account references
    let product = &mut ctx.accounts.product;
    let authority = &ctx.accounts.authority;
    
    // Initialize product account
    product.id = product_id;
    product.name = name.clone();
    product.description = description.clone();
    product.ticket_cost = ticket_cost;
    product.total_quantity = total_quantity;
    product.redeemed_quantity = 0; // No redemptions yet
    product.is_active = true; // Product is immediately available
    product.authority = authority.key();
    product.bump = ctx.bumps.product;
    
    // Log product creation details
    msg!("✅ Product added successfully");
    msg!("   Product Address: {}", product.key());
    msg!("   Authority: {}", authority.key());
    msg!("   Available Quantity: {}", product.remaining_quantity());
    msg!("   Is Available: {}", product.is_available());
    
    // Calculate economics for logging
    let total_ticket_value = ticket_cost
        .checked_mul(total_quantity as u64)
        .ok_or(ErrorCode::MathOverflow)?;
    
    msg!("📊 Product Economics:");
    msg!("   Individual Cost: {} tickets", ticket_cost);
    msg!("   Total Inventory Value: {} tickets", total_ticket_value);
    msg!("   Redemption Rate: {:.2}%", 
         (product.redeemed_quantity as f64 / product.total_quantity as f64) * 100.0);
    
    Ok(())
}
