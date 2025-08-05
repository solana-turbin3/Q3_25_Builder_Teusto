use anchor_lang::prelude::*;

// Import our modules
pub mod constants;
pub mod error;
pub mod state;
pub mod instructions;

// Import instruction handlers
use instructions::*;

declare_id!("88wXaSWyCWCYBs7sRS1KPKYVatK48YFDMh3iEovYihCv");

#[program]
pub mod simple_vote {
    use super::*;

    // Create a new poll with question, options, and duration
    pub fn create_poll(
        ctx: Context<CreatePoll>,
        poll_id: u64,
        question: String,
        options: Vec<String>,
        duration_seconds: i64,
    ) -> Result<()> {
        ctx.accounts.create_poll(poll_id, question, options, duration_seconds, &ctx.bumps)
    }

    // Cast a vote on an existing poll
    pub fn cast_vote(
        ctx: Context<CastVote>,
        option_index: u8,
    ) -> Result<()> {
        ctx.accounts.cast_vote(option_index, &ctx.bumps)
    }

    // Close a poll (creator only)
    pub fn close_poll(ctx: Context<ClosePoll>) -> Result<()> {
        ctx.accounts.close_poll()
    }
}
