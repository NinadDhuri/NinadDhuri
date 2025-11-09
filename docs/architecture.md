# Position Management System Architecture

This repository contains a reference implementation of a high-level position management stack for a perpetual futures exchange built on Solana. The goal is to demonstrate how on-chain position accounting, risk calculations, and off-chain services interact to deliver a safe trading experience.

## Components

### 1. Anchor Program (`programs/position_manager`)
* Maintains canonical position state and user collateral locks.
* Enforces leverage tiers, margin requirements, and liquidation price calculations with fixed-point math.
* Emits structured events for backend services and indexers.
* Supports position lifecycle instructions (`open_position`, `modify_position`, `close_position`).

### 2. Rust Backend (`backend/position-manager-service`)
* Provides REST APIs for opening, modifying, querying, and closing positions.
* Wraps Solana RPC interactions (stubbed in this reference) with margin analytics and persistence abstractions.
* Publishes margin alerts via broadcast channels and exposes hooks for websocket streaming.
* Tracks leverage tiers and risk metrics in sync with the on-chain program.

### 3. Persistence Layer
* `PositionRepository` trait abstracts database or cache backends.
* `InMemoryPositionRepository` gives a test-friendly implementation; swap with PostgreSQL/Redis adapters in production.
* Supports historical snapshots, owner lookups, and aggregate collateral views.

### 4. Monitoring & Metrics
* `PositionMonitor` polls open positions and pushes margin ratio alerts to the liquidation engine.
* `MetricsEmitter` fan-outs alerts to websocket clients, analytics pipelines, or paging systems.

### Data Flow

1. **Open Position**
   1. Client submits REST request → backend assembles Anchor transaction.
   2. On success, Anchor program initializes PDA-backed `Position` account and locks margin in the user account.
   3. Event stream updates backend cache and notifies monitors.

2. **Modify Position**
   * Backend validates leverage tier, recalculates entry price, and adjusts collateral.
   * Anchor program enforces invariant checks atomically before updating position state.

3. **Close Position**
   * On-chain logic realizes PnL, unlocks collateral, and zeroes liquidation triggers.
   * Off-chain services persist history, release risk reservations, and surface analytics.

## Position Lifecycle State Machine

```
OPENING -> OPEN -> MODIFYING -> OPEN -> CLOSING -> CLOSED
                       \
                        -> LIQUIDATING (liquidator takes over)
```

Each transition is atomic at the Solana program level. Off-chain services observe events to keep caches and alerts aligned.
