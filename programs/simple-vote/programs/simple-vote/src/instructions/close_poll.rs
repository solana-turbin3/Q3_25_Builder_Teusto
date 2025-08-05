use anchor_lang::prelude::*;
use crate::{constants::*, error::VoteError, state::Poll};

// Accounts needed for closing a poll
#[derive(Accounts)]
pub struct ClosePoll<'info> {
    // The poll creator (must sign the transaction)
    #[account(mut)]
    pub creator: Signer<'info>,
    
    // The poll to be closed (must be owned by the creator)
    #[account(
        mut,
        has_one = creator,                      // Verify creator ownership
        seeds = [POLL_SEED, creator.key().as_ref(), poll.poll_id.to_le_bytes().as_ref()],
        bump
    )]
    pub poll: Account<'info, Poll>,
}

impl<'info> ClosePoll<'info> {
    pub fn close_poll(&mut self) -> Result<()> {
        // Check if poll is already closed
        if !self.poll.is_active {
            return Err(VoteError::PollEnded.into());
        }
        
        // Get current time
        let current_time = Clock::get()?.unix_timestamp;
        
        // Check if poll has naturally expired
        let has_expired = current_time >= self.poll.end_time;
        
        // Allow closing if:
        // 1. Poll has naturally expired, OR
        // 2. Creator wants to close early (we'll allow this for flexibility)
        
        // Mark poll as inactive
        self.poll.is_active = false;
        
        // Log the poll results
        msg!("Poll closed successfully!");
        msg!("Poll ID: {}", self.poll.poll_id);
        msg!("Total votes: {}", self.poll.total_votes);
        msg!("Closed by creator: {}", self.creator.key());
        msg!("Closed at: {}", current_time);
        msg!("Was expired: {}", has_expired);
        
        // Log the results for each option
        for (index, (option, votes)) in self.poll.options.iter().zip(self.poll.vote_counts.iter()).enumerate() {
            msg!("Option {}: '{}' - {} votes", index, option, votes);
        }
        
        // Announce the winner if there are votes
        if let Some((winner_index, winner_votes)) = self.poll.get_winner() {
            msg!("Winner: '{}' with {} votes!", 
                self.poll.options[winner_index], 
                winner_votes
            );
        } else {
            msg!("No votes were cast on this poll.");
        }
        
        Ok(())
    }
}
