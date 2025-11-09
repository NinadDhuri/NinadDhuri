use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum PositionSide {
    Long,
    Short,
}

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenPositionRequest {
    pub symbol: String,
    pub side: PositionSide,
    #[serde_as(as = "DisplayFromStr")]
    pub size: Decimal,
    pub leverage: u16,
    #[serde_as(as = "DisplayFromStr")]
    pub entry_price: Decimal,
    pub client_position_id: u64,
}

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModifyPositionRequest {
    pub position_id: u64,
    pub action: ModifyAction,
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub execution_price: Option<Decimal>,
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub delta_size: Option<Decimal>,
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub margin_delta: Option<Decimal>,
    #[serde_as(as = "DisplayFromStr")]
    pub funding_payment: Decimal,
}

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClosePositionRequest {
    pub position_id: u64,
    #[serde_as(as = "DisplayFromStr")]
    pub execution_price: Decimal,
    #[serde_as(as = "DisplayFromStr")]
    pub funding_payment: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModifyAction {
    IncreaseSize,
    DecreaseSize,
    AddMargin,
    RemoveMargin,
}

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionView {
    pub position_id: u64,
    pub owner: String,
    pub symbol: String,
    pub side: PositionSide,
    #[serde_as(as = "DisplayFromStr")]
    pub size: Decimal,
    #[serde_as(as = "DisplayFromStr")]
    pub entry_price: Decimal,
    #[serde_as(as = "DisplayFromStr")]
    pub margin: Decimal,
    pub leverage: u16,
    #[serde_as(as = "DisplayFromStr")]
    pub unrealized_pnl: Decimal,
    #[serde_as(as = "DisplayFromStr")]
    pub realized_pnl: Decimal,
    #[serde_as(as = "DisplayFromStr")]
    pub funding_accrued: Decimal,
    #[serde_as(as = "DisplayFromStr")]
    pub liquidation_price: Decimal,
    pub last_update: DateTime<Utc>,
}

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarginSummary {
    #[serde_as(as = "DisplayFromStr")]
    pub initial_margin: Decimal,
    #[serde_as(as = "DisplayFromStr")]
    pub maintenance_margin: Decimal,
    #[serde_as(as = "DisplayFromStr")]
    pub margin_ratio: Decimal,
}

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserRiskView {
    pub owner: String,
    #[serde_as(as = "DisplayFromStr")]
    pub total_collateral: Decimal,
    #[serde_as(as = "DisplayFromStr")]
    pub locked_collateral: Decimal,
    #[serde_as(as = "DisplayFromStr")]
    pub total_pnl: Decimal,
    #[serde_as(as = "DisplayFromStr")]
    pub maintenance_margin: Decimal,
    #[serde_as(as = "DisplayFromStr")]
    pub margin_ratio: Decimal,
    pub positions: HashMap<String, Vec<PositionView>>,
}
