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
    
}