use anchor_lang::prelude::*;

// Now we need token-related types
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{close_account, transfer, CloseAccount, Mint, Token, TokenAccount, Transfer},
};

// Import our program's state and constants
use crate::{constants::SEED, state::Escrow};

// This struct defines what accounts the 'take' instruction needs
#[derive(Accounts)]
pub struct Take<'info> {
    // The person fulfilling the escrow (must sign the transaction)
    #[account(mut)] // mut because they'll pay for account creation if needed
    pub taker: Signer<'info>,
    
    // The original maker (will receive payment)
    #[account(mut)] // mut because they'll receive SOL when accounts are closed
    pub maker: SystemAccount<'info>,
    
    // The token the maker offered (what taker will receive)
    pub mint_a: Account<'info, Mint>,
    
    // The token the maker wants (what taker will provide)
    pub mint_b: Account<'info, Mint>,
    
    // Taker's token account for mint_a (where they'll receive the deposited tokens)
    #[account(
        init_if_needed,                    // Create if it doesn't exist
        payer = taker,                     // Taker pays for creation
        associated_token::mint = mint_a,   // For mint_a tokens
        associated_token::authority = taker, // Owned by taker
    )]
    pub taker_ata_a: Account<'info, TokenAccount>,
    
    // Taker's token account for mint_b (where they'll send payment from)
    #[account(
        mut,                               // We'll transfer from here
        associated_token::mint = mint_b,   // For mint_b tokens
        associated_token::authority = taker, // Owned by taker
    )]
    pub taker_ata_b: Account<'info, TokenAccount>,
    
    // Maker's token account for mint_b (where they'll receive payment)
    #[account(
        init_if_needed,                    // Create if it doesn't exist
        payer = taker,                     // Taker pays for creation
        associated_token::mint = mint_b,   // For mint_b tokens
        associated_token::authority = maker, // Owned by maker
    )]
    pub maker_ata_b: Account<'info, TokenAccount>,
    
    // The existing escrow account (will be closed and rent returned to maker)
    #[account(
        mut,                               // We'll close this account
        close = maker,                     // Return rent to maker
        has_one = maker,                   // Verify this escrow belongs to this maker
        has_one = mint_a,                  // Verify this escrow is for mint_a
        has_one = mint_b,                  // Verify this escrow is for mint_b
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

// Implementation block for the Take instruction
impl<'info> Take<'info> {
    pub fn take(&mut self) -> Result<()> {
        // Step 1: Transfer mint_b tokens from taker to maker (payment)
        let transfer_to_maker = Transfer {
            from: self.taker_ata_b.to_account_info(),    // From taker's mint_b account
            to: self.maker_ata_b.to_account_info(),      // To maker's mint_b account
            authority: self.taker.to_account_info(),     // Taker authorizes
        };

        let ctx = CpiContext::new(
            self.token_program.to_account_info(),
            transfer_to_maker,
        );

        // Transfer the amount the maker requested
        transfer(ctx, self.escrow.receive)?;

        // Step 2: Transfer mint_a tokens from vault to taker (delivery)
        let transfer_to_taker = Transfer {
            from: self.vault.to_account_info(),          // From vault
            to: self.taker_ata_a.to_account_info(),      // To taker's mint_a account
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
            transfer_to_taker,
            signer_seeds,
        );

        // Transfer all tokens from vault to taker
        transfer(ctx, self.vault.amount)?;

        // Step 3: Close the vault account (return rent to maker)
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