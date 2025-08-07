# 🏦 Solana Staking dApp - Comprehensive Implementation

A production-ready staking decentralized application built with Anchor framework for Solana blockchain.

## 📋 Table of Contents
- [Overview](#overview)
- [Architecture](#architecture)
- [Core Concepts](#core-concepts)
- [Mathematical Foundation](#mathematical-foundation)
- [Security Features](#security-features)
- [Instructions](#instructions)
- [Development Setup](#development-setup)
- [Testing](#testing)
- [Deployment](#deployment)

## 🎯 Overview

This staking dApp allows users to:
- **Stake tokens** for a specified lock period
- **Earn rewards** proportional to stake amount and time
- **Claim rewards** without unstaking
- **Unstake tokens** after lock period expires
- **View real-time** staking statistics and rewards

### Key Features
- ✅ **Time-locked staking** with configurable lock periods
- ✅ **Continuous reward accrual** calculated per second
- ✅ **Proportional rewards** based on stake size and duration
- ✅ **Flexible reward claiming** without unstaking
- ✅ **Comprehensive error handling** with detailed error codes
- ✅ **Mathematical precision** using 128-bit integers
- ✅ **Security-first design** with extensive validation

## 🏗️ Architecture

### Account Structure
```
🏦 StakingPool (Master Account)
├── 📊 Pool Configuration (authority, mints, rates)
├── 🔢 Global State (total staked, reward calculations)
└── 🏛️ Token Vaults (stake vault, reward vault)

👤 UserStake (Per-User Detail Account)
├── 💰 Stake Information (amount, lock period)
├── 🎁 Reward Tracking (earned, claimed)
└── ⏰ Time Management (stake time, unlock time)
```

### PDA Structure
```rust
// Pool PDA: ["pool", authority, pool_id]
// User Stake PDA: ["stake", pool, user]
// Stake Vault PDA: ["stake_vault", pool]
// Reward Vault PDA: ["reward_vault", pool]
```

## 🧠 Core Concepts

### 1. **Pool-Based Staking**
- Single pool manages multiple user stakes
- Shared reward calculations for efficiency
- Configurable reward rates and lock periods

### 2. **Time-Locked Mechanism**
- Mandatory lock periods prevent early withdrawal
- Higher rewards for longer commitment
- Unlock time calculated: `stake_time + lock_duration`

### 3. **Continuous Reward Accrual**
- Rewards accumulate every second
- Proportional to stake amount and time
- Fair distribution among all participants

## 🧮 Mathematical Foundation

### Core Reward Formula
```rust
// Global reward per token calculation
reward_per_token = previous_reward_per_token + 
                  (reward_rate × time_elapsed × PRECISION) ÷ total_staked

// Individual user rewards
user_rewards = stake_amount × 
              (current_reward_per_token - user_last_reward_per_token) ÷ PRECISION
```

### Precision Handling
- **Reward calculations**: 1e18 precision (128-bit integers)
- **Reward rates**: 1e9 precision (64-bit integers)
- **Prevents rounding errors** in long-term calculations

### APR Conversion
```rust
// Convert Annual Percentage Rate to reward rate per second
reward_rate = (APR ÷ 100) ÷ (365 × 24 × 60 × 60) × RATE_PRECISION

// Example: 10% APR = ~317 tokens per second per billion staked tokens
```

## 🔒 Security Features

### Input Validation
- ✅ **Stake amount limits** (min: 1 token, max: 1M tokens)
- ✅ **Lock duration bounds** (min: 1 day, max: 365 days)
- ✅ **Reward rate limits** (prevents excessive inflation)
- ✅ **Timestamp validation** (prevents time manipulation)

### Mathematical Safety
- ✅ **Overflow protection** using checked arithmetic
- ✅ **Division by zero** prevention
- ✅ **Precision loss** mitigation with large integers

### Access Control
- ✅ **Pool authority** validation for admin functions
- ✅ **User ownership** verification for stake operations
- ✅ **Account initialization** checks

### Economic Security
- ✅ **Reward vault balance** validation before payouts
- ✅ **Lock period enforcement** prevents early withdrawal
- ✅ **Dust attack prevention** with minimum stake amounts

## 📋 Instructions

### 1. `initialize_pool`
**Purpose**: Create a new staking pool
```rust
pub fn initialize_pool(
    ctx: Context<InitializePool>,
    pool_id: u64,        // Unique pool identifier
    reward_rate: u64,    // Tokens per second per staked token (scaled)
    lock_duration: i64,  // Lock period in seconds
) -> Result<()>
```

### 2. `stake`
**Purpose**: Deposit tokens into the pool
```rust
pub fn stake(
    ctx: Context<Stake>,
    amount: u64,         // Amount of tokens to stake
) -> Result<()>
```

### 3. `unstake`
**Purpose**: Withdraw tokens after lock period
```rust
pub fn unstake(ctx: Context<Unstake>) -> Result<()>
```

### 4. `claim_rewards`
**Purpose**: Claim earned rewards without unstaking
```rust
pub fn claim_rewards(ctx: Context<ClaimRewards>) -> Result<()>
```

### 5. `update_pool`
**Purpose**: Refresh reward calculations
```rust
pub fn update_pool(ctx: Context<UpdatePool>) -> Result<()>
```

## 🛠️ Development Setup

### Prerequisites
- Rust 1.70+
- Solana CLI 1.16+
- Anchor CLI 0.28+
- Node.js 18+

### Installation
```bash
# Clone and navigate to project
cd programs/staking

# Install dependencies
npm install

# Build the program
anchor build

# Run tests
anchor test
```

### Project Structure
```
src/
├── lib.rs              # Main program entry point
├── state.rs            # Account structures and logic
├── constants.rs        # Configuration and PDA seeds
├── error.rs            # Custom error definitions
└── instructions/       # Instruction handlers
    ├── mod.rs
    ├── initialize_pool.rs
    ├── stake.rs
    ├── unstake.rs
    ├── claim_rewards.rs
    └── update_pool.rs
```

## 🧪 Testing Strategy

### Test Coverage
- ✅ **Happy path scenarios** for all instructions
- ✅ **Error conditions** and edge cases
- ✅ **Mathematical accuracy** of reward calculations
- ✅ **Time-based logic** with various lock periods
- ✅ **Security validations** and access controls

### Test Categories
1. **Pool Management Tests**
   - Pool initialization
   - Invalid configurations
   - Authority validation

2. **Staking Operation Tests**
   - Valid staking scenarios
   - Amount validation
   - Balance checks

3. **Reward Calculation Tests**
   - Mathematical accuracy
   - Time-based accrual
   - Precision handling

4. **Security Tests**
   - Access control
   - Input validation
   - Error handling

## 🚀 Deployment

### Local Development
```bash
# Start local validator
solana-test-validator

# Deploy program
anchor deploy
```

### Mainnet Deployment
```bash
# Set cluster to mainnet
solana config set --url mainnet-beta

# Deploy with proper keypair
anchor deploy --provider.cluster mainnet
```

## 📊 Usage Examples

### Initialize a Pool (10% APR, 7-day lock)
```typescript
const rewardRate = aprToRewardRate(10); // 10% APR
const lockDuration = 7 * 24 * 60 * 60;  // 7 days

await program.methods
  .initializePool(poolId, rewardRate, lockDuration)
  .accounts({ /* ... */ })
  .rpc();
```

### Stake Tokens
```typescript
const stakeAmount = 1000 * 10**6; // 1000 tokens (6 decimals)

await program.methods
  .stake(new BN(stakeAmount))
  .accounts({ /* ... */ })
  .rpc();
```

### Check Rewards
```typescript
const userStake = await program.account.userStake.fetch(userStakePda);
const pendingRewards = userStake.calculatePendingRewards(currentRewardPerToken);
```

## 🔮 Future Enhancements

### Phase 2 Features
- [ ] **Multiple reward tokens** support
- [ ] **Compound staking** (auto-reinvest rewards)
- [ ] **Flexible lock periods** per user
- [ ] **Governance integration** (voting power based on stake)

### Phase 3 Features
- [ ] **Liquid staking** (tradeable stake certificates)
- [ ] **Yield farming** (multiple pool rewards)
- [ ] **NFT staking** support
- [ ] **Cross-chain** compatibility

## 📚 Learning Outcomes

By building this staking dApp, you'll master:

### Solana Development
- ✅ **Advanced PDA usage** for deterministic addresses
- ✅ **Token program integration** for vault management
- ✅ **Time-based logic** using Solana Clock
- ✅ **Mathematical operations** with precision handling

### DeFi Concepts
- ✅ **Yield generation** mechanisms
- ✅ **Tokenomics design** and incentive structures
- ✅ **Risk management** in financial protocols
- ✅ **Economic security** considerations

### Software Engineering
- ✅ **Modular architecture** design
- ✅ **Comprehensive testing** strategies
- ✅ **Error handling** best practices
- ✅ **Documentation** and code organization

---

## 📞 Support

For questions or issues:
- Review the comprehensive test suite
- Check error codes in `error.rs`
- Examine state logic in `state.rs`
- Study instruction handlers for implementation details
