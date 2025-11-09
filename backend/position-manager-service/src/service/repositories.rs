use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use rust_decimal::Decimal;
use tokio::sync::RwLock;

use super::models::{PositionView, UserRiskView};

#[async_trait]
pub trait PositionRepository: Send + Sync {
    async fn upsert_position(&self, position: PositionView) -> anyhow::Result<()>;
    async fn remove_position(&self, position_id: u64) -> anyhow::Result<()>;
    async fn get_position(&self, position_id: u64) -> anyhow::Result<Option<PositionView>>;
    async fn positions_by_owner(&self, owner: &str) -> anyhow::Result<Vec<PositionView>>;
    async fn user_risk_view(&self, owner: &str) -> anyhow::Result<Option<UserRiskView>>;
}

#[derive(Clone, Default)]
pub struct InMemoryPositionRepository {
    positions: Arc<RwLock<HashMap<u64, PositionView>>>,
}

#[async_trait]
impl PositionRepository for InMemoryPositionRepository {
    async fn upsert_position(&self, position: PositionView) -> anyhow::Result<()> {
        self.positions
            .write()
            .await
            .insert(position.position_id, position);
        Ok(())
    }

    async fn remove_position(&self, position_id: u64) -> anyhow::Result<()> {
        self.positions.write().await.remove(&position_id);
        Ok(())
    }

    async fn get_position(&self, position_id: u64) -> anyhow::Result<Option<PositionView>> {
        Ok(self.positions.read().await.get(&position_id).cloned())
    }

    async fn positions_by_owner(&self, owner: &str) -> anyhow::Result<Vec<PositionView>> {
        let positions = self
            .positions
            .read()
            .await
            .values()
            .filter(|p| p.owner == owner)
            .cloned()
            .collect();
        Ok(positions)
    }

    async fn user_risk_view(&self, owner: &str) -> anyhow::Result<Option<UserRiskView>> {
        let positions = self.positions_by_owner(owner).await?;
        if positions.is_empty() {
            return Ok(None);
        }
        let mut grouped = HashMap::new();
        for position in &positions {
            grouped
                .entry(position.symbol.clone())
                .or_insert_with(Vec::new)
                .push(position.clone());
        }
        let totals = positions.iter().fold(
            (Decimal::ZERO, Decimal::ZERO, Decimal::ZERO),
            |(collateral, locked, pnl), position| {
                (
                    collateral + position.margin,
                    locked + position.margin,
                    pnl + position.realized_pnl,
                )
            },
        );
        Ok(Some(UserRiskView {
            owner: owner.to_string(),
            total_collateral: totals.0,
            locked_collateral: totals.1,
            total_pnl: totals.2,
            maintenance_margin: Decimal::ZERO,
            margin_ratio: Decimal::ZERO,
            positions: grouped,
        }))
    }
}
