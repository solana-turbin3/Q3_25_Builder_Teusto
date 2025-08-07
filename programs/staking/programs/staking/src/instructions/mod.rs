// Export all instruction modules

pub mod initialize_pool;
pub mod stake;
pub mod unstake;
pub mod claim_rewards;
pub mod update_pool;

// Re-export the instruction structs for easy access
pub use initialize_pool::*;
pub use stake::*;
pub use unstake::*;
pub use claim_rewards::*;
pub use update_pool::*;
