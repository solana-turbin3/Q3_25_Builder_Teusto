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

/// Claim accumulated rewards without unstaking
/// Allows users to harvest rewards while keeping tokens staked
#[derive(Accounts)]
pub struct ClaimRewards<'info> {
    /// The user claiming rewards
    /// Must be the owner of the stake account
    #[account(mut)]
    pub user: Signer<'info>,

    /// The staking pool to claim rewards from
    /// Must be properly initialized
    #[account(mut)]
    pub pool: Account<'info, StakingPool>,

    /// User's stake account that tracks rewards
    /// Must belong to the user and be active
    #[account(
        mut,
        constraint = user_stake.user == user.key() @ StakingError::InvalidAccount,
        constraint = user_stake.pool == pool.key() @ StakingError::InvalidAccount,
        constraint = user_stake.is_active @ StakingError::InactiveStake,
    )]
    pub user_stake: Account<'info, UserStake>,

    /// User's token account to receive reward tokens
    /// Must be for the correct reward mint and owned by user
    #[account(
        mut,
        constraint = user_reward_token_account.mint == pool.reward_mint @ StakingError::InvalidTokenMint,
        constraint = user_reward_token_account.owner == user.key() @ StakingError::InvalidTokenAccountOwner,
    )]
    pub user_reward_token_account: Account<'info, TokenAccount>,

    /// Pool's reward vault containing reward tokens
    #[account(
        mut,
        constraint = reward_vault.key() == pool.reward_vault @ StakingError::InvalidTokenAccount,
    )]
    pub reward_vault: Account<'info, TokenAccount>,

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

impl<'info> ClaimRewards<'info> {
    /// Execute the reward claiming operation
    pub fn claim_rewards(&mut self) -> Result<()> {
        let current_time = Clock::get()?.unix_timestamp;

        // Validate that reward claiming is allowed
        self.validate_claim(current_time)?;

        // Update pool rewards to get accurate calculations
        self.update_pool_rewards(current_time)?;

        // Calculate total claimable rewards
        let claimable_rewards = self.calculate_claimable_rewards()?;

        // Transfer reward tokens to user (if any)
        if claimable_rewards > 0 {
            self.transfer_reward_tokens(claimable_rewards)?;
        }

        // Update user stake reward tracking
        self.update_user_reward_tracking(claimable_rewards)?;

        // Log the claim event
        self.log_claim_event(claimable_rewards, current_time)?;

        Ok(())
    }

    /// Validate that the reward claim operation is allowed
    fn validate_claim(&self, current_time: i64) -> Result<()> {
        let user_stake = &self.user_stake;

        // Check if stake is active
        if !user_stake.is_active {
            return Err(StakingError::InactiveStake.into());
        }

        // Check if user has any tokens staked
        if user_stake.amount == 0 {
            return Err(StakingError::NoActiveStake.into());
        }

        // Validate timestamp
        crate::error::validate_timestamp(current_time)?;

        msg!(
            "Claim validation passed: stake_amount={}, is_active={}",
            user_stake.amount,
            user_stake.is_active
        );

        Ok(())
    }

    /// Update pool reward calculations before claiming
    fn update_pool_rewards(&mut self, current_time: i64) -> Result<()> {
        let pool = &mut self.pool;

        // Calculate new reward per token
        let new_reward_per_token = pool.calculate_reward_per_token(current_time);

        // Update pool state
        pool.reward_per_token_stored = new_reward_per_token;
        pool.last_update_time = current_time;

        msg!(
            "Pool rewards updated for claim: reward_per_token={}, time={}",
            new_reward_per_token,
            current_time
        );

        Ok(())
    }

    /// Calculate the total claimable rewards for the user
    fn calculate_claimable_rewards(&mut self) -> Result<u64> {
        let pool = &self.pool;
        let user_stake = &mut self.user_stake;

        // Calculate pending rewards using current reward_per_token
        let pending_rewards = user_stake.calculate_pending_rewards(pool.reward_per_token_stored);

        // Add to existing unclaimed rewards
        let total_claimable = user_stake.rewards
            .checked_add(pending_rewards)
            .ok_or(StakingError::RewardCalculationOverflow)?;

        msg!(
            "Claimable rewards calculated: existing={}, pending={}, total={}",
            user_stake.rewards,
            pending_rewards,
            total_claimable
        );

        Ok(total_claimable)
    }

    /// Transfer reward tokens to user
    fn transfer_reward_tokens(&self, amount: u64) -> Result<()> {
        // Check if there are rewards to transfer
        if amount == 0 {
            msg!("No rewards to claim");
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

    /// Update user stake reward tracking after claiming
    fn update_user_reward_tracking(&mut self, claimed_amount: u64) -> Result<()> {
        let pool = &self.pool;
        let user_stake = &mut self.user_stake;

        // Reset rewards to zero since they've been claimed
        user_stake.rewards = 0;

        // Update the reward baseline to current reward_per_token
        user_stake.reward_per_token_paid = pool.reward_per_token_stored;

        msg!(
            "User reward tracking updated: claimed={}, new_baseline={}",
            claimed_amount,
            user_stake.reward_per_token_paid
        );

        Ok(())
    }

    /// Log the reward claim event for monitoring and analytics
    fn log_claim_event(&self, claimed_amount: u64, current_time: i64) -> Result<()> {
        let pool = &self.pool;
        let user_stake = &self.user_stake;

        // Calculate time since stake was created
        let staking_duration = current_time - user_stake.stake_time;
        let staking_days = staking_duration / (24 * 60 * 60);

        msg!(
            "CLAIM EVENT: user={}, pool={}, claimed_amount={}, stake_amount={}, staking_days={}",
            self.user.key(),
            pool.key(),
            claimed_amount,
            user_stake.amount,
            staking_days
        );

        // Calculate current APR if we have meaningful data
        if staking_duration > 0 && user_stake.amount > 0 {
            let current_apr = self.calculate_current_apr(
                user_stake.amount,
                claimed_amount,
                staking_duration,
            );
            msg!(
                "Current APR performance: {}% (pool rate: {}%)",
                current_apr,
                reward_rate_to_apr(pool.reward_rate)
            );
        }

        msg!(
            "Pool status: total_staked={}, reward_vault_balance={}",
            pool.total_staked,
            self.reward_vault.amount
        );

        Ok(())
    }

    /// Calculate the current APR achieved by the user
    fn calculate_current_apr(&self, stake_amount: u64, rewards: u64, duration_seconds: i64) -> u64 {
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

    /// Get claim summary for display
    pub fn get_claim_summary(&self, current_time: i64) -> ClaimSummary {
        let user_stake = &self.user_stake;
        let pool = &self.pool;

        // Calculate pending rewards
        let current_reward_per_token = pool.calculate_reward_per_token(current_time);
        let pending_rewards = user_stake.calculate_pending_rewards(current_reward_per_token);
        let total_claimable = user_stake.rewards + pending_rewards;

        // Calculate staking duration
        let staking_duration = current_time - user_stake.stake_time;

        ClaimSummary {
            existing_rewards: user_stake.rewards,
            pending_rewards,
            total_claimable,
            stake_amount: user_stake.amount,
            staking_duration_days: staking_duration / (24 * 60 * 60),
            reward_vault_balance: 0, // Would need to be passed in or fetched
        }
    }
}

/// Summary information about a reward claim operation
#[derive(Debug, Clone)]
pub struct ClaimSummary {
    pub existing_rewards: u64,
    pub pending_rewards: u64,
    pub total_claimable: u64,
    pub stake_amount: u64,
    pub staking_duration_days: i64,
    pub reward_vault_balance: u64,
}

/// Calculate pending rewards for a user stake
pub fn calculate_pending_rewards(
    user_stake: &UserStake,
    pool: &StakingPool,
    current_time: i64,
) -> u64 {
    let current_reward_per_token = pool.calculate_reward_per_token(current_time);
    let pending = user_stake.calculate_pending_rewards(current_reward_per_token);
    user_stake.rewards + pending
}

/// Check if a user has claimable rewards
pub fn has_claimable_rewards(
    user_stake: &UserStake,
    pool: &StakingPool,
    current_time: i64,
) -> bool {
    let total_rewards = calculate_pending_rewards(user_stake, pool, current_time);
    total_rewards > 0
}

/// Validate that a user can claim rewards
pub fn can_user_claim_rewards(user_stake: &UserStake, current_time: i64) -> Result<()> {
    if !user_stake.is_active {
        return Err(StakingError::InactiveStake.into());
    }

    if user_stake.amount == 0 {
        return Err(StakingError::NoActiveStake.into());
    }

    crate::error::validate_timestamp(current_time)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_pending_rewards() {
        // Create mock user stake
        let user_stake = UserStake {
            user: Pubkey::default(),
            pool: Pubkey::default(),
            amount: 1000 * 10_u64.pow(6), // 1000 tokens
            reward_per_token_paid: 0,
            rewards: 50 * 10_u64.pow(6), // 50 tokens existing rewards
            stake_time: 1000000,
            unlock_time: 1000000 + DEFAULT_LOCK_DURATION,
            is_active: true,
            bump: 0,
        };

        // Create mock pool
        let pool = StakingPool {
            authority: Pubkey::default(),
            stake_mint: Pubkey::default(),
            reward_mint: Pubkey::default(),
            stake_vault: Pubkey::default(),
            reward_vault: Pubkey::default(),
            reward_rate: apr_to_reward_rate(10), // 10% APR
            total_staked: 1000 * 10_u64.pow(6),
            last_update_time: 1000000,
            reward_per_token_stored: 0,
            lock_duration: DEFAULT_LOCK_DURATION,
            is_active: true,
            created_at: 1000000,
            bump: 0,
        };

        let current_time = 1000000 + (30 * 24 * 60 * 60); // 30 days later
        let total_rewards = calculate_pending_rewards(&user_stake, &pool, current_time);

        // Should have existing rewards plus some pending rewards
        assert!(total_rewards >= user_stake.rewards);
        assert!(total_rewards > 0);
    }

    #[test]
    fn test_has_claimable_rewards() {
        // Create mock data (simplified)
        let user_stake = UserStake {
            user: Pubkey::default(),
            pool: Pubkey::default(),
            amount: 1000 * 10_u64.pow(6),
            reward_per_token_paid: 0,
            rewards: 100 * 10_u64.pow(6), // Has existing rewards
            stake_time: 1000000,
            unlock_time: 1000000 + DEFAULT_LOCK_DURATION,
            is_active: true,
            bump: 0,
        };

        let pool = StakingPool {
            authority: Pubkey::default(),
            stake_mint: Pubkey::default(),
            reward_mint: Pubkey::default(),
            stake_vault: Pubkey::default(),
            reward_vault: Pubkey::default(),
            reward_rate: apr_to_reward_rate(10),
            total_staked: 1000 * 10_u64.pow(6),
            last_update_time: 1000000,
            reward_per_token_stored: 0,
            lock_duration: DEFAULT_LOCK_DURATION,
            is_active: true,
            created_at: 1000000,
            bump: 0,
        };

        let current_time = 1000000 + (7 * 24 * 60 * 60); // 7 days later

        // Should have claimable rewards
        assert!(has_claimable_rewards(&user_stake, &pool, current_time));
    }

    #[test]
    fn test_can_user_claim_rewards_validation() {
        let current_time = 1000000;
        
        // Create mock user stake
        let mut user_stake = UserStake {
            user: Pubkey::default(),
            pool: Pubkey::default(),
            amount: 1000 * 10_u64.pow(6),
            reward_per_token_paid: 0,
            rewards: 0,
            stake_time: current_time - 1000,
            unlock_time: current_time + 1000,
            is_active: true,
            bump: 0,
        };

        // Should be able to claim
        assert!(can_user_claim_rewards(&user_stake, current_time).is_ok());

        // Inactive stake should fail
        user_stake.is_active = false;
        assert!(can_user_claim_rewards(&user_stake, current_time).is_err());
        user_stake.is_active = true;

        // Zero amount should fail
        user_stake.amount = 0;
        assert!(can_user_claim_rewards(&user_stake, current_time).is_err());
    }
}
