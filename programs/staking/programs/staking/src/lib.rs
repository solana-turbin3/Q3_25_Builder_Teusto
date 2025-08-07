use anchor_lang::prelude::*;

// Import our modules
pub mod constants;
pub mod error;
pub mod state;
pub mod instructions;

// Import instruction handlers
use instructions::*;

declare_id!("AtrNJXgaUTAdrgyN8iUjAdydLZJ5s27ZEk92DiXHQ7Rh");

#[program]
pub mod staking {
    use super::*;

    /// Initialize a new staking pool with specified parameters
    /// This creates the master pool account and associated token vaults
    pub fn initialize_pool(
        ctx: Context<InitializePool>,
        pool_id: u64,
        reward_rate: u64,
        lock_duration: i64,
    ) -> Result<()> {
        ctx.accounts.initialize_pool(pool_id, reward_rate, lock_duration, &ctx.bumps)
    }

    /// Stake tokens into a pool
    /// Creates a user stake account and transfers tokens to the pool vault
    pub fn stake(
        ctx: Context<Stake>,
        amount: u64,
    ) -> Result<()> {
        ctx.accounts.stake(amount, &ctx.bumps)
    }

    /// Unstake tokens from a pool (after lock period)
    /// Calculates final rewards and transfers tokens back to user
    pub fn unstake(ctx: Context<Unstake>) -> Result<()> {
        ctx.accounts.unstake()
    }

    /// Claim accumulated rewards without unstaking
    /// Allows users to harvest rewards while keeping tokens staked
    pub fn claim_rewards(ctx: Context<ClaimRewards>) -> Result<()> {
        ctx.accounts.claim_rewards()
    }

    /// Update pool reward calculations
    /// Should be called periodically to keep reward calculations accurate
    pub fn update_pool(ctx: Context<UpdatePool>) -> Result<()> {
        ctx.accounts.update_pool()
    }
}
