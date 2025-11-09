use crate::error::PositionError;

pub fn mul_div_u128(a: u128, b: u128, denominator: u128) -> Result<u128, PositionError> {
    a.checked_mul(b)
        .ok_or(PositionError::MathOverflow)?
        .checked_div(denominator)
        .ok_or(PositionError::MathOverflow)
}

pub fn div_ceil(numerator: u128, denominator: u128) -> Result<u128, PositionError> {
    if denominator == 0 {
        return Err(PositionError::MathOverflow);
    }
    let quotient = numerator
        .checked_add(denominator - 1)
        .ok_or(PositionError::MathOverflow)?
        .checked_div(denominator)
        .ok_or(PositionError::MathOverflow)?;
    Ok(quotient)
}

pub fn checked_add_i64(left: i64, right: i64) -> Result<i64, PositionError> {
    left.checked_add(right).ok_or(PositionError::MathOverflow)
}

pub fn checked_sub_i64(left: i64, right: i64) -> Result<i64, PositionError> {
    left.checked_sub(right).ok_or(PositionError::MathOverflow)
}

pub fn checked_add_u64(left: u64, right: u64) -> Result<u64, PositionError> {
    left.checked_add(right).ok_or(PositionError::MathOverflow)
}

pub fn checked_sub_u64(left: u64, right: u64) -> Result<u64, PositionError> {
    left.checked_sub(right).ok_or(PositionError::MathOverflow)
}

pub fn checked_mul_u64(left: u64, right: u64) -> Result<u64, PositionError> {
    left.checked_mul(right).ok_or(PositionError::MathOverflow)
}

pub fn checked_div_u64(left: u64, right: u64) -> Result<u64, PositionError> {
    if right == 0 {
        return Err(PositionError::MathOverflow);
    }
    left.checked_div(right).ok_or(PositionError::MathOverflow)
}
