use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct UserState {
    pub bump: u8,
    pub points: u32,
    pub amount_staked: u8,
}