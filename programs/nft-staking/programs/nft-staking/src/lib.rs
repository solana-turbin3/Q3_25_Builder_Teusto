use anchor_lang::prelude::*;

declare_id!("GRsjtj5TQwRuXsNfsG6A7mP39ccuNCiaU86FiSGMKAiG");

#[program]
pub mod nft_staking {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        msg!("Greetings from: {:?}", ctx.program_id);
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize {}
