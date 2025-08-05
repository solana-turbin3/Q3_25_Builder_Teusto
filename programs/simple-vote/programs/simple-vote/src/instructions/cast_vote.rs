use anchor_lang::prelude::*;
use crate::{constants::*, error::VoteError, state::{Poll, VoteReceipt}};

// Accounts needed for casting a vote
#[derive(Accounts)]
pub struct CastVote<'info> {
    // The person casting the vote (must sign the transaction)
    #[account(mut)]
    pub voter: Signer<'info>,
    
    // The poll being voted on (will be modified to increment vote count)
    #[account(
        mut,
        seeds = [POLL_SEED, poll.creator.as_ref(), poll.poll_id.to_le_bytes().as_ref()],
        bump
    )]
    pub poll: Account<'info, Poll>,
    
    // Vote receipt PDA - proves this user voted (prevents double voting)
    #[account(
        init,                                    // Create new vote receipt
        payer = voter,                          // Voter pays for account creation
        space = 8 + VoteReceipt::INIT_SPACE,   // 8 bytes discriminator + receipt data
        seeds = [VOTE_SEED, poll.key().as_ref(), voter.key().as_ref()],
        bump                                    // Anchor finds the canonical bump
    )]
    pub vote_receipt: Account<'info, VoteReceipt>,
    
    // Required system program for account creation
    pub system_program: Program<'info, System>,
}

impl<'info> CastVote<'info> {
    pub fn cast_vote(
        &mut self,
        option_index: u8,
        bumps: &CastVoteBumps,
    ) -> Result<()> {
        // Validate that voting is still open
        if !self.poll.is_voting_open() {
            return Err(VoteError::PollNotActive.into());
        }
        
        // Validate the option index
        if !self.poll.is_valid_option(option_index) {
            return Err(VoteError::InvalidOption.into());
        }
        
        // Get current time
        let current_time = Clock::get()?.unix_timestamp;
        
        // Create the vote receipt (this also prevents double voting since
        // the PDA will fail to create if it already exists)
        self.vote_receipt.set_inner(VoteReceipt {
            poll: self.poll.key(),
            voter: self.voter.key(),
            option_index,
            voted_at: current_time,
        });
        
        // Increment the vote count for the chosen option
        self.poll.vote_counts[option_index as usize] += 1;
        
        // Increment total vote count
        self.poll.total_votes += 1;
        
        msg!("Vote cast successfully!");
        msg!("Voter: {}", self.voter.key());
        msg!("Poll: {}", self.poll.key());
        msg!("Option index: {}", option_index);
        msg!("Option: {}", self.poll.options[option_index as usize]);
        msg!("New vote count for this option: {}", self.poll.vote_counts[option_index as usize]);
        msg!("Total votes in poll: {}", self.poll.total_votes);
        
        Ok(())
    }
}
