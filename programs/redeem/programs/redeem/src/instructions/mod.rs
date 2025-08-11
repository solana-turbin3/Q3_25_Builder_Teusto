/// Instructions module for the Redeem program
/// 
/// This module exports all instruction handlers for the ticket exchange system.
/// Each instruction is implemented in its own file for better organization and maintainability.

pub mod initialize;
pub mod purchase_tickets;
pub mod add_product;
pub mod redeem_product;

// Re-export instruction handlers for use in lib.rs
pub use initialize::*;
pub use purchase_tickets::*;
pub use add_product::*;
pub use redeem_product::*;
