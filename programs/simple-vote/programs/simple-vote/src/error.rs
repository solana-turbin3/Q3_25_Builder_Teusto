use anchor_lang::prelude::*;

// Custom error types for our voting system
#[error_code]
pub enum VoteError {
    #[msg("Poll is not active or has expired")]
    PollNotActive,
    
    #[msg("Invalid option index provided")]
    InvalidOption,
    
    #[msg("User has already voted on this poll")]
    AlreadyVoted,
    
    #[msg("Poll duration is too short (minimum 1 hour)")]
    PollDurationTooShort,
    
    #[msg("Poll duration is too long (maximum 30 days)")]
    PollDurationTooLong,
    
    #[msg("Question is too long (maximum 200 characters)")]
    QuestionTooLong,
    
    #[msg("Option text is too long (maximum 50 characters)")]
    OptionTooLong,
    
    #[msg("Too many options provided (maximum 10)")]
    TooManyOptions,
    
    #[msg("At least 2 options are required")]
    NotEnoughOptions,
    
    #[msg("Only the poll creator can perform this action")]
    UnauthorizedCreator,
    
    #[msg("Poll has already ended")]
    PollEnded,
    
    #[msg("Cannot close poll before end time")]
    PollStillActive,
    
    #[msg("Vote counts and options length mismatch")]
    VoteCountMismatch,
}