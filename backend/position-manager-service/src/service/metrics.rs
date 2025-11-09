use rust_decimal::Decimal;
use tokio::sync::broadcast;

#[derive(Clone, Debug)]
pub struct MarginAlert {
    pub position_id: u64,
    pub margin_ratio: Decimal,
}

#[derive(Clone)]
pub struct MetricsEmitter {
    sender: broadcast::Sender<MarginAlert>,
}

impl MetricsEmitter {
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(1024);
        Self { sender }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<MarginAlert> {
        self.sender.subscribe()
    }

    pub fn emit(&self, alert: MarginAlert) {
        let _ = self.sender.send(alert);
    }
}

pub async fn monitor_margin_stream(mut receiver: broadcast::Receiver<MarginAlert>) {
    while let Ok(alert) = receiver.recv().await {
        tracing::warn!(
            position_id = alert.position_id,
            ratio = %alert.margin_ratio,
            "position approaching liquidation"
        );
    }
}
