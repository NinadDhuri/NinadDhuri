# Risk Management Overview

The position management system enforces solvency by coordinating on-chain invariants with off-chain monitoring services.

## Leverage Tiers

Both the Anchor program and backend service share the same leverage tier matrix:

| Tier | Max Leverage | Initial Margin | Maintenance Margin | Max Notional (USDT) |
| ---- | ------------ | -------------- | ------------------ | -------------------- |
| 1    | 20x          | 5.0%           | 2.5%               | Unlimited            |
| 2    | 50x          | 2.0%           | 1.0%               | 100,000              |
| 3    | 100x         | 1.0%           | 0.5%               | 50,000               |
| 4    | 500x         | 0.5%           | 0.25%              | 20,000               |
| 5    | 1000x        | 0.2%           | 0.1%               | 5,000                |

Requests breaching the configured bounds fail before collateral is locked.

## Margin Enforcement

* **Initial Margin** is locked atomically during `open_position` and when increasing exposure.
* **Maintenance Margin** drives liquidation checks; margin removal is blocked if it would violate the maintenance threshold.
* **Margin Ratio Monitoring** is surfaced through the `MetricsEmitter` which emits alerts when ratios fall below 15%.

## Liquidation Process

1. Backend calculates real-time margin ratios using oracle prices.
2. Alerts trigger the liquidation engine to submit a closing transaction.
3. Anchor program recalculates notional, realizes PnL, applies funding, and releases collateral.

## Funding

Funding payments are provided as signed integers per modification. They affect the tracked `funding_accrued` field and realized PnL to keep long/short imbalances in sync with the funding engine.

## Data Consistency

* PDA seeds tie `Position` accounts to `(owner, position_id)` pairs, ensuring deterministic addressing.
* Repository interfaces support audit trails and reconciliation jobs across Solana RPC responses and database snapshots.
* Event emitters allow stateless indexers to rebuild the full position history if needed.
