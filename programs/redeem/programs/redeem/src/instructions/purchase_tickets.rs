use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount, MintTo, mint_to};
use anchor_spl::associated_token::AssociatedToken;
use crate::state::*;
use crate::constants::*;

/// Purchase tickets with SOL
/// 
/// This instruction allows users to invest SOL and receive ticket tokens:
/// 1. Validates the purchase amount and system status
/// 2. Transfers SOL from user to the system vault
/// 3. Mints ticket tokens to the user's token account
/// 4. Creates/updates user's ticket account with balance and history
/// 5. Updates system statistics
#[derive(Accounts)]
pub struct PurchaseTickets<'info> {
    /// User purchasing tickets
    /// Must sign and pay for the tickets
    #[account(mut)]
    pub user: Signer<'info>,

    /// Main system state (PDA)
    /// Contains exchange rate and system configuration
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

    /// User's ticket account (PDA) - tracks balance and history
    /// Created automatically if it doesn't exist
    /// 
    /// Seeds: ["user_redeem", user.key()]
    /// Space: UserRedeemAccount::LEN
    #[account(
        init_if_needed,
        payer = user,
        space = 8 + UserRedeemAccount::LEN,
        seeds = [USER_REDEEM_SEED, user.key().as_ref()],
        bump
    )]
    pub user_redeem_account: Account<'info, UserRedeemAccount>,

    /// Ticket token mint (validates it matches system)
    /// 
    /// Constraint: Must match the mint in system state
    #[account(
        mut,
        constraint = ticket_mint.key() == redeem.ticket_mint @ ErrorCode::InvalidProduct
    )]
    pub ticket_mint: Account<'info, Mint>,

    /// User's SPL token account for tickets
    /// Created automatically if it doesn't exist
    /// 
    /// Associated token account: user + ticket_mint
    #[account(
        init_if_needed,
        payer = user,
        associated_token::mint = ticket_mint,
        associated_token::authority = user
    )]
    pub user_ticket_token_account: Account<'info, TokenAccount>,

    /// SOL vault that collects payments (PDA)
    /// 
    /// Seeds: ["sol_vault", redeem.key()]
    /// Constraint: Must match vault in system state
    #[account(
        mut,
        seeds = [SOL_VAULT_SEED, redeem.key().as_ref()],
        bump,
        constraint = sol_vault.key() == redeem.sol_vault @ ErrorCode::InvalidProduct
    )]
    pub sol_vault: SystemAccount<'info>,

    /// Required system programs
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub rent: Sysvar<'info, Rent>,
}

/// Purchase tickets instruction handler
/// 
/// # Arguments
/// * `ctx` - The instruction context containing all accounts
/// * `ticket_amount` - Number of tickets to purchase
/// 
/// # Security Checks
/// 1. Validates ticket amount is within bounds
/// 2. Ensures system is active
/// 3. Verifies user has sufficient SOL
/// 4. Checks for math overflow in cost calculation
/// 
/// # Process Flow
/// 1. Calculate total SOL cost
/// 2. Transfer SOL from user to vault
/// 3. Mint ticket tokens to user
/// 4. Update user account (balance, history, timestamps)
/// 5. Update system statistics
pub fn handler(ctx: Context<PurchaseTickets>, ticket_amount: u64) -> Result<()> {
    msg!("🎫 Processing ticket purchase");
    msg!("   User: {}", ctx.accounts.user.key());
    msg!("   Tickets requested: {}", ticket_amount);
    
    // Validate ticket amount
    require!(
        is_valid_ticket_amount(ticket_amount),
        ErrorCode::InvalidTicketAmount
    );
    
    // Get account references
    let redeem = &mut ctx.accounts.redeem;
    let user_redeem_account = &mut ctx.accounts.user_redeem_account;
    let user = &ctx.accounts.user;
    let ticket_mint = &ctx.accounts.ticket_mint;
    let user_ticket_token_account = &ctx.accounts.user_ticket_token_account;
    let sol_vault = &ctx.accounts.sol_vault;
    
    // Calculate total SOL cost with overflow protection
    let total_cost = redeem.calculate_sol_cost(ticket_amount)?;
    
    msg!("   Total cost: {} lamports ({} SOL)", 
         total_cost, 
         total_cost as f64 / 1_000_000_000.0);
    
    // Verify user has sufficient SOL balance
    let user_balance = user.lamports();
    require!(
        user_balance >= total_cost,
        ErrorCode::InsufficientTickets // Reusing error for insufficient funds
    );
    
    // Transfer SOL from user to vault
    let transfer_instruction = anchor_lang::system_program::Transfer {
        from: user.to_account_info(),
        to: sol_vault.to_account_info(),
    };
    
    anchor_lang::system_program::transfer(
        CpiContext::new(
            ctx.accounts.system_program.to_account_info(),
            transfer_instruction,
        ),
        total_cost,
    )?;
    
    msg!("✅ SOL transfer completed: {} lamports", total_cost);
    
    // Mint ticket tokens to user's token account
    // Use redeem PDA as mint authority
    let redeem_key = redeem.key();
    let signer_seeds: &[&[&[u8]]] = &[&[
        REDEEM_SEED,
        &[redeem.bump],
    ]];
    
    let mint_instruction = MintTo {
        mint: ticket_mint.to_account_info(),
        to: user_ticket_token_account.to_account_info(),
        authority: redeem.to_account_info(),
    };
    
    mint_to(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            mint_instruction,
            signer_seeds,
        ),
        ticket_amount,
    )?;
    
    msg!("✅ Minted {} tickets to user", ticket_amount);
    
    // Initialize user account if this is their first purchase
    if user_redeem_account.user == Pubkey::default() {
        user_redeem_account.user = user.key();
        user_redeem_account.ticket_balance = 0;
        user_redeem_account.total_purchased = 0;
        user_redeem_account.total_redeemed = 0;
        user_redeem_account.products_redeemed = 0;
        user_redeem_account.created_at = Clock::get()?.unix_timestamp;
        user_redeem_account.is_active = true;
        user_redeem_account.bump = ctx.bumps.user_redeem_account;
        
        msg!("🆕 Created new user account");
    }
    
    // Update user account with new tickets
    user_redeem_account.add_tickets(ticket_amount)?;
    
    // Update system statistics
    redeem.total_tickets_minted = redeem.total_tickets_minted
        .checked_add(ticket_amount)
        .ok_or(ErrorCode::MathOverflow)?;
    
    msg!("📊 Updated system statistics:");
    msg!("   User balance: {} tickets", user_redeem_account.ticket_balance);
    msg!("   User total purchased: {} tickets", user_redeem_account.total_purchased);
    msg!("   System total minted: {} tickets", redeem.total_tickets_minted);
    
    Ok(())
}
