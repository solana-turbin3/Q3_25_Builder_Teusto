use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct StakeState {
    pub bump: u8,
    pub owner: Pubkey,
    pub mint: Pubkey,
    pub staked_at: i64,
}