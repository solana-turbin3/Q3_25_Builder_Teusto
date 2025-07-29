#![allow(unexpected_cfgs, deprecated)]

use anchor_lang::prelude::*;
pub mod constants;
pub mod error;
pub mod instructions;
pub mod state;
pub use constants::*;
pub use instructions::*;
pub use state::*;

declare_id!("GRsjtj5TQwRuXsNfsG6A7mP39ccuNCiaU86FiSGMKAiG");

#[program]
pub mod anchor_staking {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        initialize_global_state::handler(ctx)
    }
}