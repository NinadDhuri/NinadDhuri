use std::sync::Arc;

use anyhow::Result;
use rust_decimal::Decimal;
use tokio::sync::RwLock;

use super::margin::MarginCalculator;
use super::models::{
    ClosePositionRequest, MarginSummary, ModifyAction, ModifyPositionRequest, OpenPositionRequest,
    PositionView,
};
use super::repositories::PositionRepository;
use super::risk::LeverageTier;

#[derive(Clone)]
pub struct PositionManager {
    repository: Arc<dyn PositionRepository>,
    margin_calculator: MarginCalculator,
    tiers: Arc<Vec<LeverageTier>>,
    default_owner: Arc<RwLock<String>>,
}

impl PositionManager {
    pub fn new(repository: Arc<dyn PositionRepository>, tiers: Vec<LeverageTier>) -> Self {
        Self {
            repository,
            margin_calculator: MarginCalculator::default(),
            tiers: Arc::new(tiers),
            default_owner: Arc::new(RwLock::new(String::from("demo-owner"))),
        }
    }

    pub async fn with_owner(&self, owner: &str) {
        *self.default_owner.write().await = owner.to_string();
    }

    pub async fn open_position(&self, request: OpenPositionRequest) -> Result<PositionView> {
        let notional = request.size * request.entry_price;
        self.validate_leverage(request.leverage, notional)?;
        let owner = self.default_owner.read().await.clone();
        let margin = self
            .margin_calculator
            .initial_margin(notional, Decimal::from(request.leverage))?;
        let liquidation_price = self.margin_calculator.liquidation_price(
            request.side,
            request.entry_price,
            Decimal::from(request.leverage),
            self.margin_calculator.maintenance_margin_ratio,
        )?;
        let position = PositionView {
            position_id: request.client_position_id,
            owner: owner.clone(),
            symbol: request.symbol.clone(),
            side: request.side,
            size: request.size,
            entry_price: request.entry_price,
            margin,
            leverage: request.leverage,
            unrealized_pnl: Decimal::ZERO,
            realized_pnl: Decimal::ZERO,
            funding_accrued: Decimal::ZERO,
            liquidation_price,
            last_update: chrono::Utc::now(),
        };
        self.repository.upsert_position(position.clone()).await?;
        Ok(position)
    }

    pub async fn modify_position(&self, request: ModifyPositionRequest) -> Result<PositionView> {
        let mut position = self
            .repository
            .get_position(request.position_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("position not found"))?;

        match request.action {
            ModifyAction::IncreaseSize => {
                let delta = request
                    .delta_size
                    .ok_or_else(|| anyhow::anyhow!("delta size required"))?;
                let execution_price = request
                    .execution_price
                    .ok_or_else(|| anyhow::anyhow!("execution price required"))?;
                let previous_size = position.size;
                let new_size = previous_size + delta;
                self.validate_leverage(position.leverage, new_size * execution_price)?;
                let total_value = previous_size * position.entry_price + delta * execution_price;
                let total_notional = new_size * execution_price;
                let new_margin = self
                    .margin_calculator
                    .initial_margin(total_notional, Decimal::from(position.leverage))?;
                position.size = new_size;
                position.entry_price = if new_size.is_zero() {
                    Decimal::ZERO
                } else {
                    total_value / new_size
                };
                position.margin = new_margin;
            }
            ModifyAction::DecreaseSize => {
                let delta = request
                    .delta_size
                    .ok_or_else(|| anyhow::anyhow!("delta size required"))?;
                let execution_price = request
                    .execution_price
                    .ok_or_else(|| anyhow::anyhow!("execution price required"))?;
                if delta >= position.size {
                    let pnl = self.margin_calculator.unrealized_pnl(
                        position.side,
                        position.size,
                        execution_price,
                        position.entry_price,
                    )?;
                    position.realized_pnl += pnl;
                    position.size = Decimal::ZERO;
                    position.margin = Decimal::ZERO;
                    position.unrealized_pnl = Decimal::ZERO;
                } else {
                    let pnl = self.margin_calculator.unrealized_pnl(
                        position.side,
                        delta,
                        execution_price,
                        position.entry_price,
                    )?;
                    position.realized_pnl += pnl;
                    let remaining_size = position.size - delta;
                    let remaining_value =
                        position.entry_price * position.size - execution_price * delta;
                    position.size = remaining_size;
                    position.entry_price = if remaining_size.is_zero() {
                        Decimal::ZERO
                    } else {
                        remaining_value / remaining_size
                    };
                }
            }
            ModifyAction::AddMargin => {
                let margin = request
                    .margin_delta
                    .ok_or_else(|| anyhow::anyhow!("margin delta required"))?;
                position.margin += margin;
            }
            ModifyAction::RemoveMargin => {
                let margin = request
                    .margin_delta
                    .ok_or_else(|| anyhow::anyhow!("margin delta required"))?;
                if margin >= position.margin {
                    position.margin = Decimal::ZERO;
                } else {
                    position.margin -= margin;
                }
            }
        }

        position.last_update = chrono::Utc::now();
        if !position.size.is_zero() {
            position.liquidation_price = self.margin_calculator.liquidation_price(
                position.side,
                position.entry_price,
                Decimal::from(position.leverage),
                self.margin_calculator.maintenance_margin_ratio,
            )?;
        } else {
            position.liquidation_price = Decimal::ZERO;
        }

        self.repository.upsert_position(position.clone()).await?;
        Ok(position)
    }

    pub async fn close_position(&self, request: ClosePositionRequest) -> Result<()> {
        self.repository.remove_position(request.position_id).await
    }

    pub async fn get_position(&self, position_id: u64) -> Result<Option<PositionView>> {
        self.repository.get_position(position_id).await
    }

    pub async fn positions_for_owner(&self, owner: &str) -> Result<Vec<PositionView>> {
        self.repository.positions_by_owner(owner).await
    }

    pub fn margin_summary(
        &self,
        collateral: Decimal,
        unrealized_pnl: Decimal,
        size: Decimal,
        mark_price: Decimal,
        leverage: u16,
    ) -> Result<MarginSummary> {
        let notional = self.margin_calculator.notional(size, mark_price)?;
        let initial = self
            .margin_calculator
            .initial_margin(notional, Decimal::from(leverage))?;
        let maintenance = self.margin_calculator.maintenance_margin(initial);
        let ratio = self
            .margin_calculator
            .margin_ratio(collateral, unrealized_pnl, notional)?;
        Ok(MarginSummary {
            initial_margin: initial,
            maintenance_margin: maintenance,
            margin_ratio: ratio,
        })
    }

    fn validate_leverage(&self, leverage: u16, notional: Decimal) -> Result<()> {
        if self.tiers.iter().any(|tier| {
            leverage as u32 <= tier.max_leverage as u32 && notional <= tier.max_position_notional
        }) {
            Ok(())
        } else {
            Err(anyhow::anyhow!("leverage exceeds tier limits"))
        }
    }
}
