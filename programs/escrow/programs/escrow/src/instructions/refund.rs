use anchor_lang::prelude::*;

// Now we need token-related types
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{close_account, transfer, CloseAccount, Mint, Token, TokenAccount, Transfer},
};

// Import our program's state and constants
use crate::{constants::SEED, state::Escrow};

// This struct defines what accounts the 'refund' instruction needs
#[derive(Accounts)]
pub struct Refund<'info> {
    // The original maker (must sign to prove ownership)
    #[account(mut)] // mut because they'll receive SOL when accounts are closed
    pub maker: Signer<'info>,
    
    // The token type that was deposited (what we're refunding)
    pub mint_a: Account<'info, Mint>,
    
    // Maker's token account where they'll receive the refunded tokens
    #[account(
        mut,                               // We'll transfer tokens to here
        associated_token::mint = mint_a,   // Must be for mint_a tokens
        associated_token::authority = maker, // Must be owned by maker
    )]
    pub maker_ata_a: Account<'info, TokenAccount>,
    
    // The existing escrow account (will be closed and rent returned to maker)
    #[account(
        mut,                               // We'll close this account
        close = maker,                     // Return rent to maker
        has_one = maker,                   // Verify this escrow belongs to this maker
        has_one = mint_a,                  // Verify this escrow is for mint_a
        seeds = [SEED.as_bytes(), maker.key().as_ref(), escrow.seed.to_le_bytes().as_ref()],
        bump = escrow.bump                 // Use the bump stored in escrow
    )]
    pub escrow: Account<'info, Escrow>,
    
    // The existing vault (will be closed and rent returned to maker)
    #[account(
        mut,                               // We'll transfer from and close this account
        associated_token::mint = mint_a,   // Must be for mint_a
        associated_token::authority = escrow, // Must be owned by escrow
    )]
    pub vault: Account<'info, TokenAccount>,
    
    // Required programs for token operations
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

// Implementation block for the Refund instruction
impl<'info> Refund<'info> {
    pub fn refund(&mut self) -> Result<()> {
        // Step 1: Transfer tokens from vault back to maker
        let transfer_accounts = Transfer {
            from: self.vault.to_account_info(),          // From vault
            to: self.maker_ata_a.to_account_info(),      // To maker's token account
            authority: self.escrow.to_account_info(),    // Escrow PDA authorizes
        };

        // Create signer seeds for the escrow PDA to authorize the transfer
        let maker_key = self.maker.key();
        let signer_seeds: &[&[&[u8]]] = &[&[
            SEED.as_bytes(),
            maker_key.as_ref(),
            &self.escrow.seed.to_le_bytes(),
            &[self.escrow.bump],
        ]];

        let ctx = CpiContext::new_with_signer(
            self.token_program.to_account_info(),
            transfer_accounts,
            signer_seeds,
        );

        // Transfer all tokens from vault back to maker
        transfer(ctx, self.vault.amount)?;

        // Step 2: Close the vault account (return rent to maker)
        let close_accounts = CloseAccount {
            account: self.vault.to_account_info(),       // Account to close
            destination: self.maker.to_account_info(),   // Where to send rent
            authority: self.escrow.to_account_info(),    // Escrow PDA authorizes
        };

        let ctx = CpiContext::new_with_signer(
            self.token_program.to_account_info(),
            close_accounts,
            signer_seeds,
        );

        close_account(ctx)
        // Note: The escrow account is closed automatically due to the 'close' constraint
    }
}