use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token};
use crate::state::*;
use crate::constants::*;

/// Initialize the redeem system with configurable exchange rate
/// 
/// This instruction sets up the entire ticket exchange system:
/// 1. Creates the main system state account (Redeem)
/// 2. Creates the ticket token mint
/// 3. Creates the SOL vault for collecting payments
/// 4. Sets the initial exchange rate and system parameters
/// 
/// Only the authority can call this instruction, and it can only be called once.
#[derive(Accounts)]
pub struct Initialize<'info> {
    /// The authority that will manage the system
    /// Must sign the transaction to prove ownership
    #[account(mut)]
    pub authority: Signer<'info>,

    /// Main system state account (PDA)
    /// This holds all global configuration and statistics
    /// 
    /// Seeds: ["redeem"]
    /// Space: Redeem::LEN (130 bytes)
    /// Payer: authority (pays for account creation)
    #[account(
        init,
        payer = authority,
        space = 8 + Redeem::LEN,
        seeds = [REDEEM_SEED],
        bump
    )]
    pub redeem: Account<'info, Redeem>,

    /// SPL Token mint for ticket tokens
    /// This is the "factory" that creates ticket tokens
    /// 
    /// Authority: redeem PDA (so only the program can mint)
    /// Decimals: 0 (tickets are whole numbers)
    /// Freeze authority: None (tickets can always be transferred)
    #[account(
        init,
        payer = authority,
        mint::decimals = 0,
        mint::authority = redeem,
        mint::freeze_authority = redeem
    )]
    pub ticket_mint: Account<'info, Mint>,

    /// SOL vault (PDA) that collects all payments
    /// This is where user SOL payments are stored
    /// 
    /// Seeds: ["sol_vault", redeem]
    /// Owner: System Program (regular SOL account)
    #[account(
        init,
        payer = authority,
        space = 0,
        seeds = [SOL_VAULT_SEED, redeem.key().as_ref()],
        bump
    )]
    pub sol_vault: SystemAccount<'info>,

    /// Required system programs
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
}

/// Initialize instruction handler
/// 
/// # Arguments
/// * `ctx` - The instruction context containing all accounts
/// * `sol_per_ticket` - Exchange rate in lamports per ticket
/// 
/// # Security Checks
/// 1. Validates exchange rate is within acceptable bounds
/// 2. Ensures authority signature
/// 3. Verifies PDA derivations are correct
/// 
/// # State Changes
/// 1. Initializes Redeem account with configuration
/// 2. Creates ticket mint with program as authority
/// 3. Creates SOL vault for payment collection
pub fn handler(ctx: Context<Initialize>, sol_per_ticket: u64) -> Result<()> {
    msg!("🏗️ Initializing Redeem System");
    
    // Validate exchange rate is within acceptable bounds
    require!(
        is_valid_sol_per_ticket(sol_per_ticket),
        ErrorCode::InvalidTicketAmount
    );
    
    // Get account references
    let redeem = &mut ctx.accounts.redeem;
    let authority = &ctx.accounts.authority;
    let ticket_mint = &ctx.accounts.ticket_mint;
    let sol_vault = &ctx.accounts.sol_vault;
    
    // Initialize the main system state
    redeem.authority = authority.key();
    redeem.ticket_mint = ticket_mint.key();
    redeem.sol_vault = sol_vault.key();
    redeem.sol_per_ticket = sol_per_ticket;
    redeem.total_tickets_minted = 0;
    redeem.total_tickets_redeemed = 0;
    redeem.is_active = true;
    redeem.bump = ctx.bumps.redeem;
    
    // Log system initialization
    msg!("✅ System initialized successfully");
    msg!("   Authority: {}", authority.key());
    msg!("   Ticket Mint: {}", ticket_mint.key());
    msg!("   SOL Vault: {}", sol_vault.key());
    msg!("   Exchange Rate: {} lamports per ticket", sol_per_ticket);
    msg!("   SOL per ticket: {} SOL", sol_per_ticket as f64 / 1_000_000_000.0);
    
    Ok(())
}
