///! These types are unused in the program, but are used to generate the IDL using shank.
use borsh::{BorshDeserialize, BorshSerialize};

#[derive(Debug, Copy, Clone, PartialEq, Eq, BorshDeserialize, BorshSerialize)]
#[repr(u64)]
pub enum MarketStatus {
    Uninitialized,
    /// All new orders, placements, and reductions are accepted. Crossing the spread is permissionless.
    Active,
    /// Only places, reductions and withdrawals are accepted.
    PostOnly,
    /// Only reductions and withdrawals are accepted.
    Paused,
    /// Only reductions and withdrawals are accepted. The market authority can forcibly cancel
    /// all orders.
    Closed,
    /// Used to signal the market to be deleted. Can only be called in a Closed state where all orders
    /// and traders are removed from the book
    Tombstoned,
}
