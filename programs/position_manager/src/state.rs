use anchor_lang::prelude::*;

pub const MAX_SYMBOL_LEN: usize = 16;
pub const PRICE_DECIMALS: u128 = 1_000_000; // 6 decimal precision for prices
pub const BASIS_POINTS_DIVISOR: u128 = 10_000;

#[account]
pub struct Position {
    pub owner: Pubkey,
    pub position_id: u64,
    pub symbol: String,
    pub side: Side,
    pub size: u64,
    pub entry_price: u64,
    pub margin: u64,
    pub leverage: u16,
    pub unrealized_pnl: i64,
    pub realized_pnl: i64,
    pub funding_accrued: i64,
    pub liquidation_price: u64,
    pub last_update: i64,
    pub bump: u8,
}

impl Position {
    pub const fn space() -> usize {
        8 + // discriminator
        32 + // owner
        8 + // position id
        4 + MAX_SYMBOL_LEN + // symbol string prefix + max length
        1 + // side
        8 + // size
        8 + // entry price
        8 + // margin
        2 + // leverage
        8 + // unrealized pnl
        8 + // realized pnl
        8 + // funding accrued
        8 + // liquidation price
        8 + // last update
        1 // bump
    }

    pub fn is_long(&self) -> bool {
        self.side == Side::Long
    }
}

#[account]
pub struct UserAccount {
    pub owner: Pubkey,
    pub total_collateral: u64,
    pub locked_collateral: u64,
    pub total_pnl: i64,
    pub position_count: u32,
    pub bump: u8,
}

impl UserAccount {
    pub const fn space() -> usize {
        8 + // discriminator
        32 + // owner
        8 + // total collateral
        8 + // locked collateral
        8 + // total pnl
        4 + // position count
        1 // bump
    }

    pub fn available_collateral(&self) -> u128 {
        let total = self.total_collateral as u128;
        let locked = self.locked_collateral as u128;
        total.saturating_sub(locked)
    }
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum Side {
    Long = 1,
    Short = 2,
}

impl Side {
    pub fn direction(&self) -> i64 {
        match self {
            Side::Long => 1,
            Side::Short => -1,
        }
    }
}
