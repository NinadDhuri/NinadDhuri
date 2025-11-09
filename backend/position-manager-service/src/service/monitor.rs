use std::time::Duration;

use anyhow::Result;
use tokio::{sync::watch, time::interval};

use super::manager::PositionManager;
use super::metrics::{MarginAlert, MetricsEmitter};
use super::models::PositionView;

#[derive(Clone)]
pub struct PositionMonitor {
    manager: PositionManager,
    metrics: MetricsEmitter,
    shutdown: watch::Receiver<bool>,
}

impl PositionMonitor {
    pub fn new(
        manager: PositionManager,
        metrics: MetricsEmitter,
        shutdown: watch::Receiver<bool>,
    ) -> Self {
        Self {
            manager,
            metrics,
            shutdown,
        }
    }

    pub async fn run(mut self) -> Result<()> {
        let mut ticker = interval(Duration::from_secs(5));
        loop {
            tokio::select! {
                _ = ticker.tick() => {
                    // Placeholder: in a real implementation we would stream all positions from the chain.
                }
                Ok(true) = self.shutdown.changed() => {
                    if *self.shutdown.borrow() {
                        break;
                    }
                }
            }
        }
        Ok(())
    }

    pub fn handle_margin_ratio(
        &self,
        position: &PositionView,
        margin_ratio: rust_decimal::Decimal,
    ) {
        if margin_ratio < rust_decimal::Decimal::new(15, 2) {
            self.metrics.emit(MarginAlert {
                position_id: position.position_id,
                margin_ratio,
            });
        }
    }
}
