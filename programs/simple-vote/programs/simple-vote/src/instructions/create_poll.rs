use anchor_lang::prelude::*;
use crate::{constants::*, error::VoteError, state::Poll};

// Accounts needed for creating a new poll
#[derive(Accounts)]
#[instruction(poll_id: u64)]
pub struct CreatePoll<'info> {
    // The person creating the poll (must sign the transaction)
    #[account(mut)]
    pub creator: Signer<'info>,
    
    // The poll account (PDA) - will be created
    #[account(
        init,                                    // Create new account
        payer = creator,                        // Creator pays for account creation
        space = 8 + Poll::INIT_SPACE,          // 8 bytes discriminator + poll data
        seeds = [POLL_SEED, creator.key().as_ref(), poll_id.to_le_bytes().as_ref()],
        bump                                    // Anchor finds the canonical bump
    )]
    pub poll: Account<'info, Poll>,
    
    // Required system program for account creation
    pub system_program: Program<'info, System>,
}

impl<'info> CreatePoll<'info> {
    pub fn create_poll(
        &mut self,
        poll_id: u64,
        question: String,
        options: Vec<String>,
        duration_seconds: i64,
        bumps: &CreatePollBumps,
    ) -> Result<()> {
        // Input validation
        self.validate_inputs(&question, &options, duration_seconds)?;
        
        // Get current time
        let current_time = Clock::get()?.unix_timestamp;
        
        // Calculate end time
        let end_time = current_time + duration_seconds;
        
        // Initialize vote counts (all start at 0)
        let vote_counts = vec![0u64; options.len()];
        
        // Set up the poll account
        self.poll.set_inner(Poll {
            creator: self.creator.key(),
            poll_id,
            question,
            options,
            vote_counts,
            end_time,
            is_active: true,
            total_votes: 0,
            created_at: current_time,
        });
        
        msg!("Poll created successfully!");
        msg!("Poll ID: {}", poll_id);
        msg!("Creator: {}", self.creator.key());
        msg!("End time: {}", end_time);
        
        Ok(())
    }
    
    // Validation helper function
    fn validate_inputs(
        &self,
        question: &str,
        options: &[String],
        duration_seconds: i64,
    ) -> Result<()> {
        // Check question length
        if question.len() > MAX_QUESTION_LENGTH {
            return Err(VoteError::QuestionTooLong.into());
        }
        
        // Check minimum options
        if options.len() < 2 {
            return Err(VoteError::NotEnoughOptions.into());
        }
        
        // Check maximum options
        if options.len() > MAX_OPTIONS_COUNT {
            return Err(VoteError::TooManyOptions.into());
        }
        
        // Check each option length
        for option in options {
            if option.len() > MAX_OPTION_LENGTH {
                return Err(VoteError::OptionTooLong.into());
            }
        }
        
        // Check poll duration
        if duration_seconds < MIN_POLL_DURATION {
            return Err(VoteError::PollDurationTooShort.into());
        }
        
        if duration_seconds > MAX_POLL_DURATION {
            return Err(VoteError::PollDurationTooLong.into());
        }
        
        Ok(())
    }
}
