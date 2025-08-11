<u>Q3_25_Builder_Teusto</u>

# 🚀 Solana Anchor Monorepo Cohort
Welcome! This repository contains all my projects, programs, and libraries developed during the <b>Turbin_Q3_25_Builders_Cohort</b> focused on Solana blockchain and the Anchor framework.

## About This Repo
This repository is a monorepo using Rust's Cargo workspaces, inspired by best practices for scalable blockchain development. Each cohort task or project is isolated as an individual Anchor program. Utilities and shared logic reside in libraries for modularity and code reuse.

## Project Structure
```
.
├── Cargo.toml
├── anchor.toml
├── programs/
│   ├── nft-staking/
│   ├── nft-marketplace/
│   └── capstone/
├── libs/
│   └── utils/
├── scripts/
└── README.md
```
## Programs & Apps
| Name        | Description | Status  |
| ----------- | ----------- | ------- |
| **escrow** | Trustless escrow system for atomic token swaps between parties. Implements secure make/take/refund mechanics with PDA-based vaults and cross-program invocations. | 🟢 Done  |
| **simple-vote** | Decentralized voting platform with poll creation, vote casting, and result tallying. Features time-based poll duration, multiple choice options, and creator-only poll management. | 🟢 Done  |
| **nft-marketplace** | NFT trading platform with listing, purchasing, and fee collection mechanisms. Integrates with Metaplex standards for secure NFT transfers and marketplace operations. | 🟢 Done  |
| **staking** | DeFi staking protocol with time-locked deposits, continuous reward accrual, and mathematical precision. Supports pool-based architecture with configurable APR and lock periods. | 🟢 Done  |
| **redeem** | Ticket token exchange system enabling SOL-to-token purchases and product redemption. Features configurable exchange rates, inventory management, and comprehensive audit trails. | 🟢 Done  |
| **nft-staking** | NFT-based staking system allowing users to stake NFTs for token rewards. Combines NFT ownership verification with reward distribution mechanisms. | 🛠️ WIP  |
| **capstone** | Represents culmination of cohort learning objectives. | 🛠️ WIP  |

## Contact
Author: @teusto<br>
LinkedIn: [Matheus Toscano](https://www.linkedin.com/in/matheus-toscano-oliveira/)<br>
Email: pteutoscano@gmail.com