use crate::state::Side;
use anchor_lang::prelude::*;

#[event]
pub struct PositionOpened {
    pub owner: Pubkey,
    pub position_id: u64,
    pub symbol: String,
    pub side: Side,
    pub size: u64,
    pub entry_price: u64,
    pub margin_locked: u64,
    pub leverage: u16,
    pub liquidation_price: u64,
}

#[event]
pub struct PositionModified {
    pub owner: Pubkey,
    pub position_id: u64,
    pub new_size: u64,
    pub new_margin: u64,
    pub realized_pnl: i64,
    pub unrealized_pnl: i64,
    pub liquidation_price: u64,
}

#[event]
pub struct PositionClosed {
    pub owner: Pubkey,
    pub position_id: u64,
    pub symbol: String,
    pub realized_pnl: i64,
    pub margin_released: u64,
    pub funding_paid: i64,
}
