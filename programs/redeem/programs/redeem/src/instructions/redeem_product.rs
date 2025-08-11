use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount, Burn, burn};
use crate::state::*;
use crate::constants::*;

/// Redeem tickets for a product
/// 
/// This instruction allows users to exchange their ticket tokens for real products:
/// 1. Validates user has sufficient tickets and product is available
/// 2. Burns ticket tokens from user's account (removes them from circulation)
/// 3. Updates user's ticket balance and redemption history
/// 4. Updates product inventory (reduces available quantity)
/// 5. Creates an immutable redemption record for audit trail
/// 6. Updates system statistics
/// 
/// This is the core value exchange of the entire system.
#[derive(Accounts)]
#[instruction(product_id: u64)]
pub struct RedeemProduct<'info> {
    /// User redeeming the product
    /// Must have sufficient tickets and sign the transaction
    #[account(mut)]
    pub user: Signer<'info>,

    /// Main system state (PDA)
    /// Used for validation and statistics updates
    /// 
    /// Seeds: ["redeem"]
    /// Constraint: System must be active
    #[account(
        mut,
        seeds = [REDEEM_SEED],
        bump = redeem.bump,
        constraint = redeem.is_active @ ErrorCode::SystemNotActive
    )]
    pub redeem: Account<'info, Redeem>,

    /// Product being redeemed (PDA)
    /// Contains cost, availability, and inventory information
    /// 
    /// Seeds: ["product", product_id]
    /// Constraints: Product must be available and in stock
    #[account(
        mut,
        seeds = [PRODUCT_SEED, product_id.to_le_bytes().as_ref()],
        bump = product.bump,
        constraint = product.is_available() @ ErrorCode::ProductNotAvailable,
        constraint = product.remaining_quantity() > 0 @ ErrorCode::ProductOutOfStock
    )]
    pub product: Account<'info, Product>,

    /// User's ticket account (PDA) - tracks balance and history
    /// Must exist and have sufficient balance
    /// 
    /// Seeds: ["user_redeem", user.key()]
    /// Constraint: User must have sufficient tickets
    #[account(
        mut,
        seeds = [USER_REDEEM_SEED, user.key().as_ref()],
        bump = user_redeem_account.bump,
        constraint = user_redeem_account.can_redeem(product.ticket_cost) @ ErrorCode::InsufficientTickets
    )]
    pub user_redeem_account: Account<'info, UserRedeemAccount>,

    /// User's SPL token account for tickets
    /// Contains the actual ticket tokens that will be burned
    /// 
    /// Constraint: Must belong to user and correct mint
    #[account(
        mut,
        constraint = user_ticket_token_account.owner == user.key() @ ErrorCode::Unauthorized,
        constraint = user_ticket_token_account.mint == redeem.ticket_mint @ ErrorCode::InvalidProduct
    )]
    pub user_ticket_token_account: Account<'info, TokenAccount>,

    /// Redemption record (PDA) - creates audit trail
    /// Each redemption gets a unique record for compliance and tracking
    /// 
    /// Seeds: ["redemption", user.key(), product_id, current_timestamp]
    /// Space: RedemptionRecord::LEN
    #[account(
        init,
        payer = user,
        space = 8 + RedemptionRecord::LEN,
        seeds = [
            REDEMPTION_SEED,
            user.key().as_ref(),
            product_id.to_le_bytes().as_ref(),
            &Clock::get()?.unix_timestamp.to_le_bytes()
        ],
        bump
    )]
    pub redemption_record: Account<'info, RedemptionRecord>,

    /// Required system programs
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
}

/// Redemption event - emitted for off-chain tracking
/// 
/// This event allows external systems to track redemptions in real-time
/// without having to scan all transactions on-chain
#[event]
pub struct ProductRedeemed {
    /// User who redeemed the product
    pub user: Pubkey,
    /// Product that was redeemed
    pub product_id: u64,
    /// Number of tickets spent
    pub tickets_used: u64,
    /// Timestamp of redemption
    pub timestamp: i64,
    /// Address of redemption record
    pub redemption_record: Pubkey,
}

/// Redeem product instruction handler
/// 
/// # Arguments
/// * `ctx` - The instruction context containing all accounts
/// * `product_id` - ID of the product being redeemed
/// 
/// # Security Checks
/// 1. Validates system is active
/// 2. Ensures product is available and in stock
/// 3. Verifies user has sufficient ticket balance
/// 4. Checks user owns the token account
/// 5. Validates all PDAs are correctly derived
/// 
/// # Process Flow
/// 1. Burn ticket tokens from user's account
/// 2. Update user's ticket balance and statistics
/// 3. Update product inventory
/// 4. Create redemption record for audit
/// 5. Update system statistics
/// 6. Emit redemption event
pub fn handler(ctx: Context<RedeemProduct>, product_id: u64) -> Result<()> {
    msg!("🎁 Processing product redemption");
    msg!("   User: {}", ctx.accounts.user.key());
    msg!("   Product ID: {}", product_id);
    
    // Get account references
    let redeem = &mut ctx.accounts.redeem;
    let product = &mut ctx.accounts.product;
    let user_redeem_account = &mut ctx.accounts.user_redeem_account;
    let user = &ctx.accounts.user;
    let user_ticket_token_account = &ctx.accounts.user_ticket_token_account;
    let redemption_record = &mut ctx.accounts.redemption_record;
    
    let ticket_cost = product.ticket_cost;
    let current_timestamp = Clock::get()?.unix_timestamp;
    
    msg!("   Product: {}", product.name);
    msg!("   Ticket Cost: {}", ticket_cost);
    msg!("   User Balance: {}", user_redeem_account.ticket_balance);
    msg!("   Remaining Stock: {}", product.remaining_quantity());
    
    // Burn ticket tokens from user's account
    // This permanently removes tokens from circulation
    let burn_instruction = Burn {
        mint: redeem.to_account_info(), // ticket_mint is owned by redeem
        from: user_ticket_token_account.to_account_info(),
        authority: user.to_account_info(),
    };
    
    burn(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            burn_instruction,
        ),
        ticket_cost,
    )?;
    
    msg!("✅ Burned {} ticket tokens", ticket_cost);
    
    // Update user's ticket account
    // This updates both balance and redemption history
    user_redeem_account.redeem_tickets(ticket_cost)?;
    
    msg!("✅ Updated user account:");
    msg!("   New balance: {}", user_redeem_account.ticket_balance);
    msg!("   Total redeemed: {}", user_redeem_account.total_redeemed);
    msg!("   Products redeemed: {}", user_redeem_account.products_redeemed);
    
    // Update product inventory
    product.redeemed_quantity = product.redeemed_quantity
        .checked_add(1)
        .ok_or(ErrorCode::MathOverflow)?;
    
    msg!("✅ Updated product inventory:");
    msg!("   Redeemed: {}/{}", product.redeemed_quantity, product.total_quantity);
    msg!("   Remaining: {}", product.remaining_quantity());
    msg!("   Still available: {}", product.is_available());
    
    // Create redemption record for audit trail
    redemption_record.user = user.key();
    redemption_record.product_id = product_id;
    redemption_record.tickets_used = ticket_cost;
    redemption_record.redeemed_at = current_timestamp;
    redemption_record.transaction_signature = [0u8; 64]; // Placeholder for tx sig
    redemption_record.is_processed = true;
    redemption_record.bump = ctx.bumps.redemption_record;
    
    msg!("✅ Created redemption record: {}", redemption_record.key());
    
    // Update system statistics
    redeem.total_tickets_redeemed = redeem.total_tickets_redeemed
        .checked_add(ticket_cost)
        .ok_or(ErrorCode::MathOverflow)?;
    
    msg!("📊 Updated system statistics:");
    msg!("   Total minted: {}", redeem.total_tickets_minted);
    msg!("   Total redeemed: {}", redeem.total_tickets_redeemed);
    msg!("   Tickets in circulation: {}", 
         redeem.total_tickets_minted - redeem.total_tickets_redeemed);
    
    // Emit redemption event for off-chain tracking
    emit!(ProductRedeemed {
        user: user.key(),
        product_id,
        tickets_used: ticket_cost,
        timestamp: current_timestamp,
        redemption_record: redemption_record.key(),
    });
    
    msg!("🎉 Product redemption completed successfully!");
    
    Ok(())
}
