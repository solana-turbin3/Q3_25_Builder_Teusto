use anchor_lang::prelude::*;

declare_id!("BodUuJxp88PH8hk8xGx6RuQuztpKFA2dfXyBLMFDUbr1");

#[program]
pub mod capstone {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        msg!("Greetings from: {:?}", ctx.program_id);
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize {}
