use anchor_lang::prelude::*;

// The Poll account stores all information about a voting poll
#[account]
#[derive(InitSpace)]
pub struct Poll {
    // Who created this poll (has special permissions)
    pub creator: Pubkey,
    
    // Unique identifier for this poll (chosen by creator)
    pub poll_id: u64,
    
    // The question being asked (e.g., "What's your favorite color?")
    #[max_len(200)] // Limit question to 200 characters
    pub question: String,
    
    // The available options to vote for (e.g., ["Red", "Blue", "Green"])
    #[max_len(10, 50)] // Max 10 options, each up to 50 characters
    pub options: Vec<String>,
    
    // Vote counts for each option (parallel to options vec)
    #[max_len(10)] // Must match options length
    pub vote_counts: Vec<u64>,
    
    // When this poll expires (Unix timestamp)
    pub end_time: i64,
    
    // Whether voting is still allowed
    pub is_active: bool,
    
    // Total number of votes cast
    pub total_votes: u64,
    
    // When this poll was created
    pub created_at: i64,
}

// Vote Receipt - proves that a user has voted on a specific poll
// This prevents double voting by creating a unique PDA per voter per poll
#[account]
#[derive(InitSpace)]
pub struct VoteReceipt {
    // Which poll this vote was cast on
    pub poll: Pubkey,
    
    // Who cast this vote
    pub voter: Pubkey,
    
    // Which option they voted for (index into poll.options)
    pub option_index: u8,
    
    // When the vote was cast
    pub voted_at: i64,
}

impl Poll {
    // Helper method to check if poll is still accepting votes
    pub fn is_voting_open(&self) -> bool {
        self.is_active && self.end_time > Clock::get().unwrap().unix_timestamp
    }
    
    // Helper method to validate option index
    pub fn is_valid_option(&self, option_index: u8) -> bool {
        (option_index as usize) < self.options.len()
    }
    
    // Helper method to get the winning option (returns index and vote count)
    pub fn get_winner(&self) -> Option<(usize, u64)> {
        if self.vote_counts.is_empty() {
            return None;
        }
        
        let mut max_votes = 0;
        let mut winner_index = 0;
        
        for (index, &votes) in self.vote_counts.iter().enumerate() {
            if votes > max_votes {
                max_votes = votes;
                winner_index = index;
            }
        }
        
        Some((winner_index, max_votes))
    }
}