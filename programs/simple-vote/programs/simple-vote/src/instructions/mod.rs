// Export all instruction modules

pub mod create_poll;
pub mod cast_vote;
pub mod close_poll;

// Re-export the instruction structs for easy access
pub use create_poll::*;
pub use cast_vote::*;
pub use close_poll::*;