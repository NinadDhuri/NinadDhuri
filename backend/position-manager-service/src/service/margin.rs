use rust_decimal::Decimal;
use thiserror::Error;

use super::models::PositionSide;

#[derive(Debug, Error)]
pub enum MarginError {
    #[error("calculation overflowed decimal bounds")]
    Overflow,
    #[error("position size must be positive for calculation")]
    InvalidSize,
}

#[derive(Debug, Clone)]
pub struct MarginCalculator {
    pub maintenance_margin_ratio: Decimal,
}

impl Default for MarginCalculator {
    fn default() -> Self {
        Self {
            maintenance_margin_ratio: Decimal::new(50, 4), // 0.5%
        }
    }
}

impl MarginCalculator {
    pub fn initial_margin(
        &self,
        notional: Decimal,
        leverage: Decimal,
    ) -> Result<Decimal, MarginError> {
        if leverage <= Decimal::ZERO {
            return Err(MarginError::InvalidSize);
        }
        Ok(notional / leverage)
    }

    pub fn maintenance_margin(&self, initial_margin: Decimal) -> Decimal {
        initial_margin * self.maintenance_margin_ratio
    }

    pub fn margin_ratio(
        &self,
        collateral: Decimal,
        unrealized_pnl: Decimal,
        position_value: Decimal,
    ) -> Result<Decimal, MarginError> {
        if position_value.is_zero() {
            return Err(MarginError::InvalidSize);
        }
        Ok((collateral + unrealized_pnl) / position_value)
    }

    pub fn unrealized_pnl(
        &self,
        side: PositionSide,
        size: Decimal,
        mark_price: Decimal,
        entry_price: Decimal,
    ) -> Result<Decimal, MarginError> {
        match side {
            PositionSide::Long => Ok(size * (mark_price - entry_price)),
            PositionSide::Short => Ok(size * (entry_price - mark_price)),
        }
    }

    pub fn liquidation_price(
        &self,
        side: PositionSide,
        entry_price: Decimal,
        leverage: Decimal,
        maintenance_margin_ratio: Decimal,
    ) -> Result<Decimal, MarginError> {
        if leverage <= Decimal::ZERO {
            return Err(MarginError::InvalidSize);
        }
        let ratio = Decimal::ONE / leverage;
        match side {
            PositionSide::Long => {
                Ok(entry_price * (Decimal::ONE - ratio + maintenance_margin_ratio))
            }
            PositionSide::Short => {
                Ok(entry_price * (Decimal::ONE + ratio - maintenance_margin_ratio))
            }
        }
    }

    pub fn notional(&self, size: Decimal, price: Decimal) -> Result<Decimal, MarginError> {
        if size <= Decimal::ZERO {
            return Err(MarginError::InvalidSize);
        }
        Ok(size * price)
    }
}
