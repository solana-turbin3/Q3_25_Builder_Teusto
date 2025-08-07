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

/// Stake tokens into a pool
/// Creates a user stake account and transfers tokens to the pool vault
#[derive(Accounts)]
pub struct Stake<'info> {
    /// The user who is staking tokens
    /// Must sign the transaction and pay for account creation
    #[account(mut)]
    pub user: Signer<'info>,

    /// The staking pool to stake into
    /// Must be active and properly initialized
    #[account(
        mut,
        constraint = pool.is_active @ StakingError::PoolNotActive,
    )]
    pub pool: Account<'info, StakingPool>,

    /// User's stake account - tracks their individual stake
    /// PDA: ["stake", pool.key(), user.key()]
    /// This ensures one stake account per user per pool
    #[account(
        init,
        payer = user,
        space = UserStake::INIT_SPACE,
        seeds = [STAKE_SEED, pool.key().as_ref(), user.key().as_ref()],
        bump
    )]
    pub user_stake: Account<'info, UserStake>,

    /// User's token account containing the tokens to stake
    /// Must have sufficient balance and be owned by the user
    #[account(
        mut,
        constraint = user_token_account.mint == pool.stake_mint @ StakingError::InvalidTokenMint,
        constraint = user_token_account.owner == user.key() @ StakingError::InvalidTokenAccountOwner,
    )]
    pub user_token_account: Account<'info, TokenAccount>,

    /// Pool's stake vault where staked tokens are held
    /// Must match the vault specified in the pool
    #[account(
        mut,
        constraint = stake_vault.key() == pool.stake_vault @ StakingError::InvalidTokenAccount,
    )]
    pub stake_vault: Account<'info, TokenAccount>,

    /// The stake token mint (for validation)
    #[account(
        constraint = stake_mint.key() == pool.stake_mint @ StakingError::InvalidTokenMint,
    )]
    pub stake_mint: Account<'info, Mint>,

    /// Required system programs
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub rent: Sysvar<'info, Rent>,
}

impl<'info> Stake<'info> {
    /// Execute the staking operation
    pub fn stake(&mut self, amount: u64, bumps: &StakeBumps) -> Result<()> {
        let current_time = Clock::get()?.unix_timestamp;

        // Validate the stake amount and user eligibility
        self.validate_stake(amount, current_time)?;

        // Update pool rewards before adding new stake
        // This ensures fair reward distribution
        self.update_pool_rewards(current_time)?;

        // Initialize the user stake account
        self.initialize_user_stake(amount, current_time, bumps)?;

        // Transfer tokens from user to pool vault
        self.transfer_tokens_to_vault(amount)?;

        // Update pool state with new stake
        self.update_pool_state(amount, current_time)?;

        // Log the staking event
        self.log_stake_event(amount, current_time)?;

        Ok(())
    }

    /// Validate that the stake operation is allowed
    fn validate_stake(&self, amount: u64, current_time: i64) -> Result<()> {
        // Check if pool allows staking
        if !self.pool.can_stake(current_time) {
            return Err(StakingError::PoolNotActive.into());
        }

        // Validate stake amount is within bounds
        if !is_valid_stake_amount(amount) {
            if amount < MIN_STAKE_AMOUNT {
                msg!(
                    "Stake amount {} is below minimum {}",
                    amount,
                    MIN_STAKE_AMOUNT
                );
                return Err(StakingError::StakeAmountTooSmall.into());
            } else {
                msg!(
                    "Stake amount {} exceeds maximum {}",
                    amount,
                    MAX_STAKE_AMOUNT
                );
                return Err(StakingError::StakeAmountTooLarge.into());
            }
        }

        // Check user has sufficient balance
        if self.user_token_account.amount < amount {
            msg!(
                "Insufficient balance: has {}, needs {}",
                self.user_token_account.amount,
                amount
            );
            return Err(StakingError::InsufficientBalance.into());
        }

        // Validate timestamp
        crate::error::validate_timestamp(current_time)?;

        Ok(())
    }

    /// Update pool reward calculations before adding new stake
    /// This ensures existing stakers get fair rewards up to this point
    fn update_pool_rewards(&mut self, current_time: i64) -> Result<()> {
        let pool = &mut self.pool;

        // Calculate new reward per token
        let new_reward_per_token = pool.calculate_reward_per_token(current_time);

        // Update pool state
        pool.reward_per_token_stored = new_reward_per_token;
        pool.last_update_time = current_time;

        msg!(
            "Pool rewards updated: reward_per_token={}, time={}",
            new_reward_per_token,
            current_time
        );

        Ok(())
    }

    /// Initialize the user stake account with appropriate values
    fn initialize_user_stake(
        &mut self,
        amount: u64,
        current_time: i64,
        bumps: &StakeBumps,
    ) -> Result<()> {
        let user_stake = &mut self.user_stake;
        let pool = &self.pool;

        // Set basic stake information
        user_stake.user = self.user.key();
        user_stake.pool = pool.key();
        user_stake.amount = amount;

        // Set reward tracking
        // User starts with current reward_per_token as their baseline
        user_stake.reward_per_token_paid = pool.reward_per_token_stored;
        user_stake.rewards = 0; // No rewards yet

        // Set time information
        user_stake.stake_time = current_time;
        user_stake.unlock_time = current_time + pool.lock_duration;

        // Set status
        user_stake.is_active = true;
        user_stake.bump = bumps.user_stake;

        msg!(
            "User stake initialized: amount={}, unlock_time={}",
            amount,
            user_stake.unlock_time
        );

        Ok(())
    }

    /// Transfer tokens from user account to pool vault
    fn transfer_tokens_to_vault(&self, amount: u64) -> Result<()> {
        // Create transfer instruction
        let transfer_ctx = CpiContext::new(
            self.token_program.to_account_info(),
            Transfer {
                from: self.user_token_account.to_account_info(),
                to: self.stake_vault.to_account_info(),
                authority: self.user.to_account_info(),
            },
        );

        // Execute the transfer
        token::transfer(transfer_ctx, amount)?;

        msg!("Transferred {} tokens to stake vault", amount);

        Ok(())
    }

    /// Update pool state after successful stake
    fn update_pool_state(&mut self, amount: u64, current_time: i64) -> Result<()> {
        let pool = &mut self.pool;

        // Add to total staked amount
        pool.total_staked = pool.total_staked
            .checked_add(amount)
            .ok_or(StakingError::MathOverflow)?;

        // Update last update time
        pool.last_update_time = current_time;

        msg!(
            "Pool state updated: total_staked={}, last_update={}",
            pool.total_staked,
            current_time
        );

        Ok(())
    }

    /// Log the staking event for monitoring and analytics
    fn log_stake_event(&self, amount: u64, current_time: i64) -> Result<()> {
        let pool = &self.pool;
        let user_stake = &self.user_stake;

        msg!(
            "STAKE EVENT: user={}, pool={}, amount={}, unlock_time={}, total_pool_staked={}",
            self.user.key(),
            pool.key(),
            amount,
            user_stake.unlock_time,
            pool.total_staked
        );

        // Calculate and log expected rewards
        let lock_duration = pool.lock_duration;
        let estimated_rewards = calculate_estimated_rewards(
            amount,
            pool.reward_rate,
            lock_duration,
        );

        msg!(
            "Expected rewards for {}-day lock: {} tokens ({}% APR)",
            lock_duration / (24 * 60 * 60),
            estimated_rewards,
            reward_rate_to_apr(pool.reward_rate)
        );

        Ok(())
    }

    /// Get stake summary for display
    pub fn get_stake_summary(&self, amount: u64) -> StakeSummary {
        let pool = &self.pool;
        let lock_days = pool.lock_duration / (24 * 60 * 60);
        let apr = reward_rate_to_apr(pool.reward_rate);
        let estimated_rewards = calculate_estimated_rewards(
            amount,
            pool.reward_rate,
            pool.lock_duration,
        );

        StakeSummary {
            stake_amount: amount,
            lock_duration_days: lock_days,
            apr_percent: apr,
            estimated_rewards,
            unlock_timestamp: Clock::get().unwrap().unix_timestamp + pool.lock_duration,
        }
    }
}

/// Summary information about a stake operation
#[derive(Debug, Clone)]
pub struct StakeSummary {
    pub stake_amount: u64,
    pub lock_duration_days: i64,
    pub apr_percent: u64,
    pub estimated_rewards: u64,
    pub unlock_timestamp: i64,
}

/// Calculate estimated rewards for a stake
/// This is the same function from initialize_pool but repeated here for convenience
pub fn calculate_estimated_rewards(
    stake_amount: u64,
    reward_rate: u64,
    time_period_seconds: i64,
) -> u64 {
    // Formula: (stake_amount * reward_rate * time_period) / RATE_PRECISION
    let rewards = (stake_amount as u128)
        .checked_mul(reward_rate as u128)
        .and_then(|x| x.checked_mul(time_period_seconds as u128))
        .and_then(|x| x.checked_div(RATE_PRECISION as u128))
        .unwrap_or(0) as u64;
    
    rewards
}

/// Validate that a user can stake in a pool
pub fn can_user_stake(
    pool: &StakingPool,
    user_balance: u64,
    stake_amount: u64,
    current_time: i64,
) -> Result<()> {
    // Check pool is active
    if !pool.can_stake(current_time) {
        return Err(StakingError::PoolNotActive.into());
    }

    // Check stake amount is valid
    if !is_valid_stake_amount(stake_amount) {
        return Err(StakingError::StakeAmountTooSmall.into());
    }

    // Check user has sufficient balance
    if user_balance < stake_amount {
        return Err(StakingError::InsufficientBalance.into());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_estimated_rewards() {
        let stake_amount = 1000 * 10_u64.pow(6); // 1000 tokens
        let reward_rate = apr_to_reward_rate(12); // 12% APR
        let lock_duration = 30 * 24 * 60 * 60; // 30 days

        let rewards = calculate_estimated_rewards(stake_amount, reward_rate, lock_duration);
        
        // 30 days should be approximately 1/12 of annual rewards
        // 12% APR for 30 days ≈ 1% of stake amount
        let expected_min = stake_amount / 120; // ~0.83%
        let expected_max = stake_amount / 80;  // ~1.25%
        
        assert!(rewards >= expected_min && rewards <= expected_max);
    }

    #[test]
    fn test_can_user_stake_validation() {
        // Create a mock pool (this would normally be more complex)
        let mut pool = StakingPool {
            authority: Pubkey::default(),
            stake_mint: Pubkey::default(),
            reward_mint: Pubkey::default(),
            stake_vault: Pubkey::default(),
            reward_vault: Pubkey::default(),
            reward_rate: apr_to_reward_rate(10),
            total_staked: 0,
            last_update_time: 0,
            reward_per_token_stored: 0,
            lock_duration: DEFAULT_LOCK_DURATION,
            is_active: true,
            created_at: 0,
            bump: 0,
        };

        let current_time = 1000000;
        let user_balance = 5000 * 10_u64.pow(6);
        let stake_amount = 1000 * 10_u64.pow(6);

        // Valid stake should pass
        assert!(can_user_stake(&pool, user_balance, stake_amount, current_time).is_ok());

        // Inactive pool should fail
        pool.is_active = false;
        assert!(can_user_stake(&pool, user_balance, stake_amount, current_time).is_err());
        pool.is_active = true;

        // Insufficient balance should fail
        assert!(can_user_stake(&pool, stake_amount - 1, stake_amount, current_time).is_err());

        // Invalid stake amount should fail
        assert!(can_user_stake(&pool, user_balance, MIN_STAKE_AMOUNT - 1, current_time).is_err());
    }

    #[test]
    fn test_stake_summary() {
        let stake_amount = 2000 * 10_u64.pow(6);
        let reward_rate = apr_to_reward_rate(15);
        let lock_duration = 7 * 24 * 60 * 60; // 7 days

        let estimated_rewards = calculate_estimated_rewards(stake_amount, reward_rate, lock_duration);
        
        // Verify the calculation makes sense
        assert!(estimated_rewards > 0);
        assert!(estimated_rewards < stake_amount); // Rewards shouldn't exceed principal for short periods
    }
}
