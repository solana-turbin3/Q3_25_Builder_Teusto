#![allow(unexpected_cfgs, deprecated)]
use anchor_lang::prelude::*;

pub mod constants;
pub mod state;
pub mod instructions;

use instructions::*;
use state::*;

declare_id!("FEUtZsWm99vwPCMuwPiKrBWg4TTSgTaqeBUsmEovhPJD");

// The #[program] macro tells Anchor that this is the entry point module containing all your instruction handlers.
#[program]
pub mod escrow_program {
    use super::*;

    pub fn make(ctx: Context<Make>, seed: u64, receive: u64, deposit: u64) -> Result<()> {
        ctx.accounts.make(seed, receive, deposit, &ctx.bumps)
    }

    pub fn take(ctx: Context<Take>) -> Result<()> {
        ctx.accounts.take()
    }
}