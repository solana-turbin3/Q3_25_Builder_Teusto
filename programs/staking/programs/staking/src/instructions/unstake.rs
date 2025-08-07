use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{self, Mint, Token, TokenAccount, Transfer},
};

use crate::{
    constants::*,
    error::StakingError,
    state::{StakingPool, UserStake},
};

/// Unstake tokens from a pool (after lock period expires)
/// Calculates final rewards and transfers tokens back to user
#[derive(Accounts)]
pub struct Unstake<'info> {
    /// The user who is unstaking tokens
    /// Must be the owner of the stake account
    #[account(mut)]
    pub user: Signer<'info>,

    /// The staking pool to unstake from
    /// Must be properly initialized
    #[account(mut)]
    pub pool: Account<'info, StakingPool>,

    /// User's stake account that will be closed
    /// Must belong to the user and be ready for unstaking
    #[account(
        mut,
        close = user,  // Close account and return rent to user
        constraint = user_stake.user == user.key() @ StakingError::InvalidAccount,
        constraint = user_stake.pool == pool.key() @ StakingError::InvalidAccount,
        constraint = user_stake.is_active @ StakingError::InactiveStake,
    )]
    pub user_stake: Account<'info, UserStake>,

    /// User's token account to receive staked tokens
    /// Must be for the correct mint and owned by user
    #[account(
        mut,
        constraint = user_stake_token_account.mint == pool.stake_mint @ StakingError::InvalidTokenMint,
        constraint = user_stake_token_account.owner == user.key() @ StakingError::InvalidTokenAccountOwner,
    )]
    pub user_stake_token_account: Account<'info, TokenAccount>,

    /// User's token account to receive reward tokens
    /// Can be the same as stake token account for single-token pools
    #[account(
        mut,
        constraint = user_reward_token_account.mint == pool.reward_mint @ StakingError::InvalidTokenMint,
        constraint = user_reward_token_account.owner == user.key() @ StakingError::InvalidTokenAccountOwner,
    )]
    pub user_reward_token_account: Account<'info, TokenAccount>,

    /// Pool's stake vault containing the staked tokens
    #[account(
        mut,
        constraint = stake_vault.key() == pool.stake_vault @ StakingError::InvalidTokenAccount,
    )]
    pub stake_vault: Account<'info, TokenAccount>,

    /// Pool's reward vault containing reward tokens
    #[account(
        mut,
        constraint = reward_vault.key() == pool.reward_vault @ StakingError::InvalidTokenAccount,
    )]
    pub reward_vault: Account<'info, TokenAccount>,

    /// The stake token mint (for validation)
    #[account(
        constraint = stake_mint.key() == pool.stake_mint @ StakingError::InvalidTokenMint,
    )]
    pub stake_mint: Account<'info, Mint>,

    /// The reward token mint (for validation)
    #[account(
        constraint = reward_mint.key() == pool.reward_mint @ StakingError::InvalidTokenMint,
    )]
    pub reward_mint: Account<'info, Mint>,

    /// Required system programs
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
}

impl<'info> Unstake<'info> {
    /// Execute the unstaking operation
    pub fn unstake(&mut self) -> Result<()> {
        let current_time = Clock::get()?.unix_timestamp;

        // Validate that unstaking is allowed
        self.validate_unstake(current_time)?;

        // Update pool rewards to get accurate final calculations
        self.update_pool_rewards(current_time)?;

        // Calculate final rewards for the user
        let final_rewards = self.calculate_final_rewards()?;

        // Get stake amount before account is closed
        let stake_amount = self.user_stake.amount;

        // Transfer staked tokens back to user
        self.transfer_staked_tokens(stake_amount)?;

        // Transfer reward tokens to user (if any)
        if final_rewards > 0 {
            self.transfer_reward_tokens(final_rewards)?;
        }

        // Update pool state after unstaking
        self.update_pool_state(stake_amount, current_time)?;

        // Log the unstaking event
        self.log_unstake_event(stake_amount, final_rewards, current_time)?;

        Ok(())
    }

    /// Validate that the unstake operation is allowed
    fn validate_unstake(&self, current_time: i64) -> Result<()> {
        let user_stake = &self.user_stake;

        // Check if stake is active
        if !user_stake.is_active {
            return Err(StakingError::InactiveStake.into());
        }

        // Check if lock period has expired
        if !user_stake.can_unstake(current_time) {
            let time_remaining = user_stake.time_until_unlock(current_time);
            msg!(
                "Stake is still locked. Time remaining: {} seconds ({} days)",
                time_remaining,
                time_remaining / (24 * 60 * 60)
            );
            return Err(StakingError::StakeStillLocked.into());
        }

        // Check if user has any tokens staked
        if user_stake.amount == 0 {
            return Err(StakingError::CannotUnstakeZero.into());
        }

        // Validate timestamp
        crate::error::validate_timestamp(current_time)?;

        msg!(
            "Unstake validation passed: amount={}, lock_expired={}",
            user_stake.amount,
            current_time >= user_stake.unlock_time
        );

        Ok(())
    }

    /// Update pool reward calculations before unstaking
    fn update_pool_rewards(&mut self, current_time: i64) -> Result<()> {
        let pool = &mut self.pool;

        // Calculate new reward per token
        let new_reward_per_token = pool.calculate_reward_per_token(current_time);

        // Update pool state
        pool.reward_per_token_stored = new_reward_per_token;
        pool.last_update_time = current_time;

        msg!(
            "Pool rewards updated for unstake: reward_per_token={}, time={}",
            new_reward_per_token,
            current_time
        );

        Ok(())
    }

    /// Calculate the final rewards earned by the user
    fn calculate_final_rewards(&mut self) -> Result<u64> {
        let pool = &self.pool;
        let user_stake = &mut self.user_stake;

        // Calculate pending rewards using current reward_per_token
        let pending_rewards = user_stake.calculate_pending_rewards(pool.reward_per_token_stored);

        // Add to existing unclaimed rewards
        let total_rewards = user_stake.rewards
            .checked_add(pending_rewards)
            .ok_or(StakingError::RewardCalculationOverflow)?;

        // Update user stake with final reward calculation
        user_stake.rewards = total_rewards;
        user_stake.reward_per_token_paid = pool.reward_per_token_stored;

        msg!(
            "Final rewards calculated: pending={}, total={}, reward_per_token={}",
            pending_rewards,
            total_rewards,
            pool.reward_per_token_stored
        );

        Ok(total_rewards)
    }

    /// Transfer staked tokens back to user
    fn transfer_staked_tokens(&self, amount: u64) -> Result<()> {
        // Check vault has sufficient balance
        if self.stake_vault.amount < amount {
            msg!(
                "Insufficient stake vault balance: has {}, needs {}",
                self.stake_vault.amount,
                amount
            );
            return Err(StakingError::InsufficientTokenBalance.into());
        }

        // Create PDA signer seeds for pool authority
        let pool_key = self.pool.key();
        let seeds = &[
            POOL_SEED,
            self.pool.authority.as_ref(),
            &pool_key.to_bytes()[..8], // Use first 8 bytes as pool_id
            &[self.pool.bump],
        ];
        let signer_seeds = &[&seeds[..]];

        // Create transfer context with pool as authority
        let transfer_ctx = CpiContext::new_with_signer(
            self.token_program.to_account_info(),
            Transfer {
                from: self.stake_vault.to_account_info(),
                to: self.user_stake_token_account.to_account_info(),
                authority: self.pool.to_account_info(),
            },
            signer_seeds,
        );

        // Execute the transfer
        token::transfer(transfer_ctx, amount)?;

        msg!("Transferred {} staked tokens back to user", amount);

        Ok(())
    }

    /// Transfer reward tokens to user
    fn transfer_reward_tokens(&self, amount: u64) -> Result<()> {
        // Check if there are rewards to transfer
        if amount == 0 {
            return Ok(());
        }

        // Check vault has sufficient balance
        if self.reward_vault.amount < amount {
            msg!(
                "Insufficient reward vault balance: has {}, needs {}",
                self.reward_vault.amount,
                amount
            );
            return Err(StakingError::InsufficientRewardTokens.into());
        }

        // Create PDA signer seeds for pool authority
        let pool_key = self.pool.key();
        let seeds = &[
            POOL_SEED,
            self.pool.authority.as_ref(),
            &pool_key.to_bytes()[..8], // Use first 8 bytes as pool_id
            &[self.pool.bump],
        ];
        let signer_seeds = &[&seeds[..]];

        // Create transfer context with pool as authority
        let transfer_ctx = CpiContext::new_with_signer(
            self.token_program.to_account_info(),
            Transfer {
                from: self.reward_vault.to_account_info(),
                to: self.user_reward_token_account.to_account_info(),
                authority: self.pool.to_account_info(),
            },
            signer_seeds,
        );

        // Execute the transfer
        token::transfer(transfer_ctx, amount)?;

        msg!("Transferred {} reward tokens to user", amount);

        Ok(())
    }

    /// Update pool state after unstaking
    fn update_pool_state(&mut self, stake_amount: u64, current_time: i64) -> Result<()> {
        let pool = &mut self.pool;

        // Subtract from total staked amount
        pool.total_staked = pool.total_staked
            .checked_sub(stake_amount)
            .ok_or(StakingError::MathOverflow)?;

        // Update last update time
        pool.last_update_time = current_time;

        msg!(
            "Pool state updated after unstake: total_staked={}, last_update={}",
            pool.total_staked,
            current_time
        );

        Ok(())
    }

    /// Log the unstaking event for monitoring and analytics
    fn log_unstake_event(
        &self,
        stake_amount: u64,
        rewards: u64,
        current_time: i64,
    ) -> Result<()> {
        let pool = &self.pool;
        let user_stake = &self.user_stake;

        // Calculate staking duration
        let staking_duration = current_time - user_stake.stake_time;
        let staking_days = staking_duration / (24 * 60 * 60);

        msg!(
            "UNSTAKE EVENT: user={}, pool={}, stake_amount={}, rewards={}, duration_days={}",
            self.user.key(),
            pool.key(),
            stake_amount,
            rewards,
            staking_days
        );

        // Calculate actual APR achieved
        if staking_duration > 0 {
            let actual_apr = self.calculate_actual_apr(stake_amount, rewards, staking_duration);
            msg!(
                "Actual APR achieved: {}% (expected: {}%)",
                actual_apr,
                reward_rate_to_apr(pool.reward_rate)
            );
        }

        msg!(
            "Pool remaining: total_staked={}, reward_vault_balance={}",
            pool.total_staked,
            self.reward_vault.amount
        );

        Ok(())
    }

    /// Calculate the actual APR achieved by the user
    fn calculate_actual_apr(&self, stake_amount: u64, rewards: u64, duration_seconds: i64) -> u64 {
        if stake_amount == 0 || duration_seconds == 0 {
            return 0;
        }

        // Convert to annual rate
        let seconds_per_year = 365 * 24 * 60 * 60;
        let annual_rewards = (rewards as u128)
            .checked_mul(seconds_per_year as u128)
            .and_then(|x| x.checked_div(duration_seconds as u128))
            .unwrap_or(0);

        // Calculate APR as percentage
        let apr = annual_rewards
            .checked_mul(100)
            .and_then(|x| x.checked_div(stake_amount as u128))
            .unwrap_or(0) as u64;

        apr
    }

    /// Get unstake summary for display
    pub fn get_unstake_summary(&self, current_time: i64) -> UnstakeSummary {
        let user_stake = &self.user_stake;
        let pool = &self.pool;

        let staking_duration = current_time - user_stake.stake_time;
        let can_unstake = user_stake.can_unstake(current_time);
        let time_until_unlock = if can_unstake { 0 } else { user_stake.time_until_unlock(current_time) };

        // Calculate pending rewards
        let current_reward_per_token = pool.calculate_reward_per_token(current_time);
        let pending_rewards = user_stake.calculate_pending_rewards(current_reward_per_token);
        let total_rewards = user_stake.rewards + pending_rewards;

        UnstakeSummary {
            stake_amount: user_stake.amount,
            total_rewards,
            staking_duration_days: staking_duration / (24 * 60 * 60),
            can_unstake,
            time_until_unlock_seconds: time_until_unlock,
        }
    }
}

/// Summary information about an unstake operation
#[derive(Debug, Clone)]
pub struct UnstakeSummary {
    pub stake_amount: u64,
    pub total_rewards: u64,
    pub staking_duration_days: i64,
    pub can_unstake: bool,
    pub time_until_unlock_seconds: i64,
}

/// Check if a user can unstake their tokens
pub fn can_user_unstake(user_stake: &UserStake, current_time: i64) -> Result<()> {
    if !user_stake.is_active {
        return Err(StakingError::InactiveStake.into());
    }

    if !user_stake.can_unstake(current_time) {
        return Err(StakingError::StakeStillLocked.into());
    }

    if user_stake.amount == 0 {
        return Err(StakingError::CannotUnstakeZero.into());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_actual_apr() {
        // Mock unstake context (simplified)
        let stake_amount = 1000 * 10_u64.pow(6); // 1000 tokens
        let rewards = 100 * 10_u64.pow(6); // 100 tokens reward
        let duration = 365 * 24 * 60 * 60; // 1 year

        // Create a mock unstake context
        let mock_unstake = Unstake {
            user: todo!(), // These would be properly initialized in real tests
            pool: todo!(),
            user_stake: todo!(),
            user_stake_token_account: todo!(),
            user_reward_token_account: todo!(),
            stake_vault: todo!(),
            reward_vault: todo!(),
            stake_mint: todo!(),
            reward_mint: todo!(),
            system_program: todo!(),
            token_program: todo!(),
            associated_token_program: todo!(),
        };

        // This test would need proper mock setup to work
        // let actual_apr = mock_unstake.calculate_actual_apr(stake_amount, rewards, duration);
        // assert_eq!(actual_apr, 10); // Should be 10% APR
    }

    #[test]
    fn test_can_user_unstake_validation() {
        let current_time = 1000000;
        
        // Create mock user stake
        let mut user_stake = UserStake {
            user: Pubkey::default(),
            pool: Pubkey::default(),
            amount: 1000 * 10_u64.pow(6),
            reward_per_token_paid: 0,
            rewards: 0,
            stake_time: current_time - 1000,
            unlock_time: current_time - 100, // Already unlocked
            is_active: true,
            bump: 0,
        };

        // Should be able to unstake
        assert!(can_user_unstake(&user_stake, current_time).is_ok());

        // Inactive stake should fail
        user_stake.is_active = false;
        assert!(can_user_unstake(&user_stake, current_time).is_err());
        user_stake.is_active = true;

        // Still locked should fail
        user_stake.unlock_time = current_time + 1000;
        assert!(can_user_unstake(&user_stake, current_time).is_err());
        user_stake.unlock_time = current_time - 100;

        // Zero amount should fail
        user_stake.amount = 0;
        assert!(can_user_unstake(&user_stake, current_time).is_err());
    }

    #[test]
    fn test_unstake_summary_calculation() {
        let current_time = 1000000;
        let stake_time = current_time - (7 * 24 * 60 * 60); // 7 days ago
        
        // This test would require proper mock setup for pool and user_stake
        // to test the get_unstake_summary method
    }
}
