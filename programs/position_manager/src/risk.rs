use crate::{
    error::PositionError,
    state::{BASIS_POINTS_DIVISOR, PRICE_DECIMALS},
};

#[derive(Clone, Copy, Debug)]
pub struct LeverageTier {
    pub max_leverage: u16,
    pub initial_margin_rate_bps: u64,
    pub maintenance_margin_rate_bps: u64,
    pub max_position_notional: u128,
}

pub const LEVERAGE_TIERS: [LeverageTier; 5] = [
    LeverageTier {
        max_leverage: 20,
        initial_margin_rate_bps: 500,
        maintenance_margin_rate_bps: 250,
        max_position_notional: u128::MAX,
    },
    LeverageTier {
        max_leverage: 50,
        initial_margin_rate_bps: 200,
        maintenance_margin_rate_bps: 100,
        max_position_notional: 100_000u128 * PRICE_DECIMALS,
    },
    LeverageTier {
        max_leverage: 100,
        initial_margin_rate_bps: 100,
        maintenance_margin_rate_bps: 50,
        max_position_notional: 50_000u128 * PRICE_DECIMALS,
    },
    LeverageTier {
        max_leverage: 500,
        initial_margin_rate_bps: 50,
        maintenance_margin_rate_bps: 25,
        max_position_notional: 20_000u128 * PRICE_DECIMALS,
    },
    LeverageTier {
        max_leverage: 1_000,
        initial_margin_rate_bps: 20,
        maintenance_margin_rate_bps: 10,
        max_position_notional: 5_000u128 * PRICE_DECIMALS,
    },
];

pub fn get_leverage_tier(
    leverage: u16,
    position_notional: u128,
) -> Result<LeverageTier, PositionError> {
    for tier in LEVERAGE_TIERS.iter() {
        if leverage <= tier.max_leverage && position_notional <= tier.max_position_notional {
            return Ok(*tier);
        }
    }
    Err(PositionError::LeverageExceeded)
}

pub fn margin_from_rate(notional: u128, rate_bps: u64) -> Result<u128, PositionError> {
    let numerator = notional
        .checked_mul(rate_bps as u128)
        .ok_or(PositionError::MathOverflow)?;
    numerator
        .checked_div(BASIS_POINTS_DIVISOR)
        .ok_or(PositionError::MathOverflow)
}
