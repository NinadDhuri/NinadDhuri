# Margin & PnL Formulas

This reference outlines the formulas implemented across the Anchor program and backend service. All calculations avoid floating-point rounding issues by using integer fixed-point arithmetic on-chain and `rust_decimal` off-chain.

## Notation

| Symbol | Description |
| ------ | ----------- |
| `S`    | Position size (base asset amount) |
| `P₀`   | Entry price |
| `Pₘ`   | Mark price |
| `L`    | Leverage (x) |
| `IMR`  | Initial margin rate (tier-based) |
| `MMR`  | Maintenance margin rate (tier-based) |
| `Q`    | Position notional value (`S × P`) |

## Initial Margin

```
InitialMargin = max(Q / L, Q × IMR)
```

On-chain the notional value is computed as `(size * price) / PRICE_DECIMALS` to maintain 6-decimal precision. Rates are stored in basis points (`1 bps = 0.01%`).

## Maintenance Margin

```
MaintenanceMargin = max(Q / L, Q × MMR)
```

Maintenance checks trigger liquidations once the margin ratio falls below the tier-specific maintenance rate.

## Margin Ratio

```
MarginRatio = (Margin + UnrealizedPnL) / (S × Pₘ)
```

If `MarginRatio < MMR`, liquidation becomes eligible.

## Unrealized PnL

```
Long:  UnrealizedPnL = S × (Pₘ - P₀)
Short: UnrealizedPnL = S × (P₀ - Pₘ)
```

## Liquidation Prices

```
Long : Pₗ = P₀ × (1 - 1/L + MMR)
Short: Pₗ = P₀ × (1 + 1/L - MMR)
```

The Anchor program performs the computation with `LIQUIDATION_PRECISION = 1_000_000_000` to preserve precision during integer division.

## Realized PnL on Partial Close

When a trader reduces the position by `ΔS` at execution price `Pₑ`:

```
RealizedPnL = ΔS × (Pₑ - P₀)    if long
RealizedPnL = ΔS × (P₀ - Pₑ)    if short
```

The backend maintains the same logic in decimal arithmetic for analytics while the smart contract ensures atomic settlement.

## Funding Payments

Funding adjustments enter the position as signed integers and contribute to both realized PnL and the tracked `funding_accrued` field. All funding payments are applied before recalculating liquidation prices.
