use anchor_lang::prelude::*;

mod error;
mod events;
mod math;
mod risk;
mod state;

use error::PositionError;
use events::{PositionClosed, PositionModified, PositionOpened};
use math::*;
use risk::{get_leverage_tier, margin_from_rate};
use state::{Position, Side, UserAccount, BASIS_POINTS_DIVISOR, MAX_SYMBOL_LEN, PRICE_DECIMALS};

declare_id!("11111111111111111111111111111111");

const LIQUIDATION_PRECISION: u128 = 1_000_000_000u128;

#[program]
pub mod position_manager {
    use super::*;

    pub fn open_position(
        ctx: Context<OpenPosition>,
        position_id: u64,
        symbol: String,
        side: Side,
        size: u64,
        leverage: u16,
        entry_price: u64,
    ) -> Result<()> {
        require!(symbol.len() <= MAX_SYMBOL_LEN, PositionError::SymbolTooLong);
        require!(
            leverage >= 1 && leverage <= 1_000,
            PositionError::InvalidLeverage
        );
        require!(size > 0, PositionError::InvalidPositionSize);

        let clock = Clock::get()?;
        let position_notional =
            math::mul_div_u128(size as u128, entry_price as u128, PRICE_DECIMALS)?;
        let tier = get_leverage_tier(leverage, position_notional)?;

        let initial_margin_from_leverage = math::div_ceil(position_notional, leverage as u128)?;
        let minimum_initial_margin =
            margin_from_rate(position_notional, tier.initial_margin_rate_bps)?;
        let required_margin = core::cmp::max(initial_margin_from_leverage, minimum_initial_margin);
        let required_margin_u64 =
            u64::try_from(required_margin).map_err(|_| PositionError::MathOverflow)?;

        let user = &mut ctx.accounts.user_account;
        require_keys_eq!(
            user.owner,
            ctx.accounts.owner.key(),
            PositionError::Unauthorized
        );
        require!(
            user.available_collateral() >= required_margin,
            PositionError::InsufficientCollateral
        );

        user.locked_collateral = checked_add_u64(user.locked_collateral, required_margin_u64)?;
        user.position_count = user.position_count.saturating_add(1);

        let position = &mut ctx.accounts.position;
        position.owner = ctx.accounts.owner.key();
        position.position_id = position_id;
        position.symbol = symbol.clone();
        position.side = side;
        position.size = size;
        position.entry_price = entry_price;
        position.margin = required_margin_u64;
        position.leverage = leverage;
        position.unrealized_pnl = 0;
        position.realized_pnl = 0;
        position.funding_accrued = 0;
        position.last_update = clock.unix_timestamp;
        position.bump = *ctx.bumps.get("position").expect("position bump");
        sync_leverage(position, position_notional)?;
        position.liquidation_price = compute_liquidation_price(
            side,
            entry_price,
            position.leverage,
            tier.maintenance_margin_rate_bps,
        )?;

        emit!(PositionOpened {
            owner: position.owner,
            position_id,
            symbol,
            side,
            size,
            entry_price,
            margin_locked: required_margin_u64,
            leverage,
            liquidation_price: position.liquidation_price,
        });

        Ok(())
    }

    pub fn modify_position(
        ctx: Context<ModifyPosition>,
        position_id: u64,
        params: ModifyPositionParams,
    ) -> Result<()> {
        let clock = Clock::get()?;
        let user = &mut ctx.accounts.user_account;
        let position = &mut ctx.accounts.position;
        require!(
            position.position_id == position_id,
            PositionError::PositionIdMismatch
        );
        require_keys_eq!(
            position.owner,
            ctx.accounts.owner.key(),
            PositionError::Unauthorized
        );

        match params.action {
            ModifyAction::IncreaseSize {
                delta_size,
                execution_price,
            } => {
                require!(delta_size > 0, PositionError::InvalidPositionSize);
                increase_position_size(position, user, delta_size, execution_price)?;
            }
            ModifyAction::DecreaseSize {
                delta_size,
                execution_price,
            } => {
                require!(delta_size > 0, PositionError::InvalidPositionSize);
                decrease_position_size(
                    position,
                    user,
                    delta_size,
                    execution_price,
                    params.funding_payment,
                )?;
            }
            ModifyAction::AddMargin { amount } => {
                add_margin(position, user, amount)?;
                apply_funding(position, params.funding_payment)?;
            }
            ModifyAction::RemoveMargin { amount } => {
                remove_margin(position, user, amount)?;
                apply_funding(position, params.funding_payment)?;
            }
        }

        position.last_update = clock.unix_timestamp;

        emit!(PositionModified {
            owner: position.owner,
            position_id: position.position_id,
            new_size: position.size,
            new_margin: position.margin,
            realized_pnl: position.realized_pnl,
            unrealized_pnl: position.unrealized_pnl,
            liquidation_price: position.liquidation_price,
        });

        Ok(())
    }

    pub fn close_position(
        ctx: Context<ClosePosition>,
        position_id: u64,
        execution_price: u64,
        funding_payment: i64,
    ) -> Result<()> {
        let clock = Clock::get()?;
        let user = &mut ctx.accounts.user_account;
        let position = &mut ctx.accounts.position;
        require!(
            position.position_id == position_id,
            PositionError::PositionIdMismatch
        );
        require_keys_eq!(
            position.owner,
            ctx.accounts.owner.key(),
            PositionError::Unauthorized
        );

        let size = position.size;
        require!(size > 0, PositionError::PositionClosed);

        let margin_before = position.margin;
        decrease_position_size(position, user, size, execution_price, funding_payment)?;
        let margin_released = margin_before
            .checked_sub(position.margin)
            .ok_or(PositionError::MathOverflow)?;

        emit!(PositionClosed {
            owner: position.owner,
            position_id: position.position_id,
            symbol: position.symbol.clone(),
            realized_pnl: position.realized_pnl,
            margin_released,
            funding_paid: funding_payment,
        });

        position.size = 0;
        position.margin = 0;
        position.leverage = 0;
        position.unrealized_pnl = 0;
        position.liquidation_price = 0;
        position.last_update = clock.unix_timestamp;

        Ok(())
    }
}

fn increase_position_size(
    position: &mut Account<Position>,
    user: &mut Account<UserAccount>,
    delta_size: u64,
    execution_price: u64,
) -> Result<()> {
    let new_size = checked_add_u64(position.size, delta_size)?;
    let current_notional = compute_notional(position.size, position.entry_price)?;
    let add_notional = compute_notional(delta_size, execution_price)?;
    let new_notional = current_notional
        .checked_add(add_notional)
        .ok_or(PositionError::MathOverflow)?;

    let tier = get_leverage_tier(position.leverage, new_notional)?;
    let required_margin_from_leverage = math::div_ceil(new_notional, position.leverage as u128)?;
    let minimum_margin = margin_from_rate(new_notional, tier.initial_margin_rate_bps)?;
    let target_margin = core::cmp::max(required_margin_from_leverage, minimum_margin);
    let target_margin_u64 =
        u64::try_from(target_margin).map_err(|_| PositionError::MathOverflow)?;

    if target_margin_u64 > position.margin {
        let delta_margin = target_margin_u64 - position.margin;
        require!(
            user.available_collateral() >= delta_margin as u128,
            PositionError::InsufficientCollateral
        );
        user.locked_collateral = checked_add_u64(user.locked_collateral, delta_margin)?;
        position.margin = checked_add_u64(position.margin, delta_margin)?;
    }

    let new_entry_price = math::mul_div_u128(new_notional, PRICE_DECIMALS, new_size as u128)?;
    position.entry_price =
        u64::try_from(new_entry_price).map_err(|_| PositionError::MathOverflow)?;
    position.size = new_size;
    position.unrealized_pnl = 0;
    sync_leverage(position, new_notional)?;
    position.liquidation_price = compute_liquidation_price(
        position.side,
        position.entry_price,
        position.leverage,
        tier.maintenance_margin_rate_bps,
    )?;

    Ok(())
}

fn decrease_position_size(
    position: &mut Account<Position>,
    user: &mut Account<UserAccount>,
    delta_size: u64,
    execution_price: u64,
    funding_payment: i64,
) -> Result<()> {
    require!(
        delta_size <= position.size,
        PositionError::PositionSizeTooSmall
    );

    let remaining_size = checked_sub_u64(position.size, delta_size)?;
    let current_notional = compute_notional(position.size, position.entry_price)?;
    let closing_notional = compute_notional(delta_size, execution_price)?;

    // Realized PnL
    let entry_notional = compute_notional(delta_size, position.entry_price)?;
    let pnl_direction = position.side.direction() as i128;
    let pnl = (closing_notional as i128)
        .checked_sub(entry_notional as i128)
        .ok_or(PositionError::MathOverflow)?
        .checked_mul(pnl_direction)
        .ok_or(PositionError::MathOverflow)?;
    let pnl_i64 = i64::try_from(pnl).map_err(|_| PositionError::MathOverflow)?;

    position.realized_pnl = checked_add_i64(position.realized_pnl, pnl_i64)?;
    user.total_pnl = checked_add_i64(user.total_pnl, pnl_i64)?;

    // Release margin proportionally
    let margin_to_release = math::mul_div_u128(
        position.margin as u128,
        delta_size as u128,
        position.size as u128,
    )?;
    let margin_to_release_u64 =
        u64::try_from(margin_to_release).map_err(|_| PositionError::MathOverflow)?;
    position.margin = checked_sub_u64(position.margin, margin_to_release_u64)?;
    user.locked_collateral = checked_sub_u64(user.locked_collateral, margin_to_release_u64)?;

    apply_funding(position, funding_payment)?;

    position.size = remaining_size;
    if remaining_size == 0 {
        position.unrealized_pnl = 0;
        position.liquidation_price = 0;
        position.leverage = 0;
    } else {
        let remaining_notional = current_notional
            .checked_sub(entry_notional)
            .ok_or(PositionError::MathOverflow)?;
        let new_entry_price =
            math::mul_div_u128(remaining_notional, PRICE_DECIMALS, remaining_size as u128)?;
        position.entry_price =
            u64::try_from(new_entry_price).map_err(|_| PositionError::MathOverflow)?;
        let tier = get_leverage_tier(position.leverage, remaining_notional)?;
        let required_margin_from_leverage =
            math::div_ceil(remaining_notional, position.leverage as u128)?;
        let minimum_margin = margin_from_rate(remaining_notional, tier.initial_margin_rate_bps)?;
        let target_margin = core::cmp::max(required_margin_from_leverage, minimum_margin);
        let target_margin_u64 =
            u64::try_from(target_margin).map_err(|_| PositionError::MathOverflow)?;

        if position.margin < target_margin_u64 {
            let shortfall = target_margin_u64 - position.margin;
            require!(
                user.available_collateral() >= shortfall as u128,
                PositionError::InsufficientCollateral
            );
            user.locked_collateral = checked_add_u64(user.locked_collateral, shortfall)?;
            position.margin = checked_add_u64(position.margin, shortfall)?;
        }
        sync_leverage(position, remaining_notional)?;
        position.liquidation_price = compute_liquidation_price(
            position.side,
            position.entry_price,
            position.leverage,
            tier.maintenance_margin_rate_bps,
        )?;
    }

    Ok(())
}

fn add_margin(
    position: &mut Account<Position>,
    user: &mut Account<UserAccount>,
    amount: u64,
) -> Result<()> {
    require!(amount > 0, PositionError::MarginTooLow);
    require!(
        user.available_collateral() >= amount as u128,
        PositionError::InsufficientCollateral
    );
    user.locked_collateral = checked_add_u64(user.locked_collateral, amount)?;
    position.margin = checked_add_u64(position.margin, amount)?;
    let notional = compute_notional(position.size, position.entry_price)?;
    sync_leverage(position, notional)?;
    Ok(())
}

fn remove_margin(
    position: &mut Account<Position>,
    user: &mut Account<UserAccount>,
    amount: u64,
) -> Result<()> {
    require!(amount > 0, PositionError::MarginTooLow);
    require!(amount <= position.margin, PositionError::MarginTooLow);

    let notional = compute_notional(position.size, position.entry_price)?;
    let tier = get_leverage_tier(position.leverage, notional)?;
    let required_margin_from_leverage = math::div_ceil(notional, position.leverage as u128)?;
    let minimum_margin = margin_from_rate(notional, tier.maintenance_margin_rate_bps)?;
    let target_margin = core::cmp::max(required_margin_from_leverage, minimum_margin);
    let target_margin_u64 =
        u64::try_from(target_margin).map_err(|_| PositionError::MathOverflow)?;

    require!(
        position.margin - amount >= target_margin_u64,
        PositionError::MarginTooLow
    );

    position.margin = checked_sub_u64(position.margin, amount)?;
    user.locked_collateral = checked_sub_u64(user.locked_collateral, amount)?;
    sync_leverage(position, notional)?;

    Ok(())
}

fn apply_funding(position: &mut Account<Position>, funding_payment: i64) -> Result<()> {
    if funding_payment == 0 {
        return Ok(());
    }
    position.funding_accrued = checked_add_i64(position.funding_accrued, funding_payment)?;
    Ok(())
}

fn compute_liquidation_price(
    side: Side,
    entry_price: u64,
    leverage: u16,
    maintenance_margin_bps: u64,
) -> Result<u64> {
    if leverage == 0 || entry_price == 0 {
        return Ok(0);
    }

    let entry_price_u128 = entry_price as u128;
    let maintenance_component = (maintenance_margin_bps as u128)
        .checked_mul(LIQUIDATION_PRECISION)
        .ok_or(PositionError::MathOverflow)?
        .checked_div(BASIS_POINTS_DIVISOR)
        .ok_or(PositionError::MathOverflow)?;
    let leverage_component = LIQUIDATION_PRECISION
        .checked_div(leverage as u128)
        .ok_or(PositionError::MathOverflow)?;

    let multiplier = match side {
        Side::Long => LIQUIDATION_PRECISION
            .checked_sub(leverage_component)
            .ok_or(PositionError::MathOverflow)?
            .checked_add(maintenance_component)
            .ok_or(PositionError::MathOverflow)?,
        Side::Short => LIQUIDATION_PRECISION
            .checked_add(leverage_component)
            .ok_or(PositionError::MathOverflow)?
            .checked_sub(maintenance_component)
            .ok_or(PositionError::MathOverflow)?,
    };

    let price = entry_price_u128
        .checked_mul(multiplier)
        .ok_or(PositionError::MathOverflow)?
        .checked_div(LIQUIDATION_PRECISION)
        .ok_or(PositionError::MathOverflow)?;

    Ok(u64::try_from(price).map_err(|_| PositionError::MathOverflow)?)
}

fn compute_notional(size: u64, price: u64) -> Result<u128> {
    if size == 0 || price == 0 {
        return Ok(0);
    }
    math::mul_div_u128(size as u128, price as u128, PRICE_DECIMALS)
}

fn sync_leverage(position: &mut Account<Position>, notional: u128) -> Result<()> {
    if position.margin == 0 || position.size == 0 {
        position.leverage = 0;
        return Ok(());
    }
    let leverage_ratio = notional
        .checked_div(position.margin as u128)
        .ok_or(PositionError::MathOverflow)?
        .min(1_000u128);
    position.leverage = u16::try_from(leverage_ratio).map_err(|_| PositionError::MathOverflow)?;
    Ok(())
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct ModifyPositionParams {
    pub action: ModifyAction,
    pub funding_payment: i64,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub enum ModifyAction {
    IncreaseSize {
        delta_size: u64,
        execution_price: u64,
    },
    DecreaseSize {
        delta_size: u64,
        execution_price: u64,
    },
    AddMargin {
        amount: u64,
    },
    RemoveMargin {
        amount: u64,
    },
}

#[derive(Accounts)]
#[instruction(position_id: u64)]
pub struct OpenPosition<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,
    #[account(
        mut,
        seeds = [b"user", owner.key().as_ref()],
        bump = user_account.bump,
    )]
    pub user_account: Account<'info, UserAccount>,
    #[account(
        init,
        payer = owner,
        space = Position::space(),
        seeds = [b"position", owner.key().as_ref(), &position_id.to_le_bytes()],
        bump,
    )]
    pub position: Account<'info, Position>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(position_id: u64)]
pub struct ModifyPosition<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,
    #[account(
        mut,
        seeds = [b"user", owner.key().as_ref()],
        bump = user_account.bump,
    )]
    pub user_account: Account<'info, UserAccount>,
    #[account(
        mut,
        has_one = owner,
        seeds = [b"position", owner.key().as_ref(), &position_id.to_le_bytes()],
        bump = position.bump,
    )]
    pub position: Account<'info, Position>,
}

#[derive(Accounts)]
#[instruction(position_id: u64)]
pub struct ClosePosition<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,
    #[account(
        mut,
        seeds = [b"user", owner.key().as_ref()],
        bump = user_account.bump,
    )]
    pub user_account: Account<'info, UserAccount>,
    #[account(
        mut,
        has_one = owner,
        seeds = [b"position", owner.key().as_ref(), &position_id.to_le_bytes()],
        bump = position.bump,
    )]
    pub position: Account<'info, Position>,
}
