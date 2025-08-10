use anchor_lang::prelude::*;

declare_id!("6GzrsBjRd5N3cMfMhoZMvEr5STWXRV8wJEYGLsAw4vAA");

#[program]
pub mod redeem {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        msg!("Greetings from: {:?}", ctx.program_id);
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize {}
