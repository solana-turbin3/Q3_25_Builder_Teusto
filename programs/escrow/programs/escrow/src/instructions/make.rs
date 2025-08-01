use anchor_lang::prelude::*;

// Now we need token-related types
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{transfer, Mint, Token, TokenAccount, Transfer},
};

// Import our program's state and constants
use crate::{constants::SEED, state::Escrow};

// This struct defines what accounts the 'make' instruction needs
#[derive(Accounts)]
#[instruction(seed: u64)] // This instruction takes a seed parameter
pub struct Make<'info> {
    // The person creating the escrow (must sign the transaction)
    #[account(mut)] // mut = mutable, because we'll deduct SOL for account creation
    pub maker: Signer<'info>,
    
    // The token the maker is offering (e.g., USDC)
    pub mint_a: Account<'info, Mint>,
    
    // The token the maker wants in return (e.g., SOL)
    pub mint_b: Account<'info, Mint>,
    
    // The maker's token account for mint_a (where they currently hold their tokens)
    #[account(
        mut,                           // We'll transfer tokens from here
        associated_token::mint = mint_a,  // Must be for mint_a
        associated_token::authority = maker, // Must be owned by maker
    )]
    pub maker_ata_a: Account<'info, TokenAccount>,
    
    // The escrow account that stores our trade details (PDA)
    #[account(
        init,                    // Create a new account
        payer = maker,          // Maker pays for account creation
        space = 8 + Escrow::INIT_SPACE, // Size: 8 bytes (discriminator) + our struct size
        seeds = [SEED.as_bytes(), maker.key().as_ref(), seed.to_le_bytes().as_ref()],
        bump                    // Anchor finds the bump for us
    )]
    pub escrow: Account<'info, Escrow>,
    
    // The vault that will hold the deposited tokens (owned by escrow PDA)
    #[account(
        init,                           // Create new token account
        payer = maker,                 // Maker pays for creation
        associated_token::mint = mint_a,   // For mint_a tokens
        associated_token::authority = escrow, // Owned by escrow PDA
    )]
    pub vault: Account<'info, TokenAccount>,
    
    // Required programs for token operations
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

// Implementation block for the Make instruction
impl<'info> Make<'info> {
    pub fn make(&mut self, seed: u64, receive: u64, deposit: u64, bumps: &MakeBumps) -> Result<()> {
        // Step 1: Initialize the escrow account with trade details
        self.escrow.set_inner(Escrow {
            seed,                           // User-provided seed
            maker: self.maker.key(),       // Who created this escrow
            mint_a: self.mint_a.key(),     // Token they're offering
            mint_b: self.mint_b.key(),     // Token they want
            receive,                       // Amount of mint_b they want
            bump: bumps.escrow,           // PDA bump for security
        });

        // Step 2: Transfer tokens from maker to vault
        let transfer_accounts = Transfer {
            from: self.maker_ata_a.to_account_info(),  // From maker's token account
            to: self.vault.to_account_info(),          // To vault
            authority: self.maker.to_account_info(),   // Maker authorizes
        };

        let ctx = CpiContext::new(
            self.token_program.to_account_info(),
            transfer_accounts,
        );

        // Execute the transfer
        transfer(ctx, deposit)
    }
}