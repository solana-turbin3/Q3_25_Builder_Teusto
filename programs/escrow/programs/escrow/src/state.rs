use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct Escrow {
    pub seed: u64, // Unique identifier for the escrow
    pub maker: Pubkey, // Person who created the escrow
    pub mint_a: Pubkey, // Token they're offering
    pub mint_b: Pubkey, // Token they're receiving in return
    pub receive: u64, // The amount of the second token to receive
    pub bump: u8, // The bump of the escrow for security
}