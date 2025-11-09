use rust_decimal::Decimal;

#[derive(Debug, Clone)]
pub struct LeverageTier {
    pub max_leverage: u16,
    pub initial_margin_rate: Decimal,
    pub maintenance_margin_rate: Decimal,
    pub max_position_notional: Decimal,
}

pub fn default_leverage_tiers() -> Vec<LeverageTier> {
    vec![
        LeverageTier {
            max_leverage: 20,
            initial_margin_rate: Decimal::new(500, 4),
            maintenance_margin_rate: Decimal::new(250, 4),
            max_position_notional: Decimal::MAX,
        },
        LeverageTier {
            max_leverage: 50,
            initial_margin_rate: Decimal::new(200, 4),
            maintenance_margin_rate: Decimal::new(100, 4),
            max_position_notional: Decimal::from(100_000u64),
        },
        LeverageTier {
            max_leverage: 100,
            initial_margin_rate: Decimal::new(100, 4),
            maintenance_margin_rate: Decimal::new(50, 4),
            max_position_notional: Decimal::from(50_000u64),
        },
        LeverageTier {
            max_leverage: 500,
            initial_margin_rate: Decimal::new(50, 4),
            maintenance_margin_rate: Decimal::new(25, 4),
            max_position_notional: Decimal::from(20_000u64),
        },
        LeverageTier {
            max_leverage: 1_000,
            initial_margin_rate: Decimal::new(20, 4),
            maintenance_margin_rate: Decimal::new(10, 4),
            max_position_notional: Decimal::from(5_000u64),
        },
    ]
}
