use anyhow::{anyhow, Result};
use rust_decimal::{
    prelude::{FromPrimitive, ToPrimitive},
    Decimal, MathematicalOps,
};

pub fn abs(tick: &str) -> Result<u32> {
    let tick_u32 = as_u32(tick)?;
    if sign(tick)? == 0 {
        Ok(tick_u32)
    } else if tick_u32 <= 1 << 31 {
        Err(anyhow!("Invalid tick value: {}", tick))
    } else {
        u32_neg(tick_u32)
    }
}

pub fn as_i32(tick: &str) -> Result<i32> {
    let tick_u32 = as_u32(tick)?;
    i32_from_u32(tick_u32)
}

pub fn i32_from_u32(tick: u32) -> Result<i32> {
    if sign_u32(tick) == 0 {
        Ok(tick as i32)
    } else {
        Ok(-(u32_neg(tick - 1)? as i32))
    }
}

pub fn sqrt_price_from_tick(tick: Decimal) -> Result<Decimal> {
    if tick < Decimal::from_i32(-tick_bound()).unwrap()
        || tick > Decimal::from_i32(tick_bound()).unwrap()
    {
        return Err(anyhow!("Tick {} out of bounds: {}", tick, tick_bound()));
    }

    let tick_f64 = tick.to_f64().ok_or_else(|| anyhow!("tick to f64 failed"))?;
    let base = 1.0001_f64;
    let sqrt_price_f64 = base.powf(tick_f64 / 2.0);
    let sqrt_price = Decimal::from_f64(sqrt_price_f64)
        .ok_or_else(|| anyhow!("sqrt_price_f64 to Decimal failed"))?;

    Ok(sqrt_price)
}

pub fn slippage_from_sqrt_price(
    current_sqrt_price: Decimal,
    target_sqrt_price: Decimal,
) -> Result<Decimal> {
    if current_sqrt_price.is_zero() {
        return Err(anyhow!("Current sqrt price cannot be zero"));
    }

    let slippage = (target_sqrt_price.powu(2) / current_sqrt_price.powu(2) - Decimal::ONE).abs();
    Ok(slippage)
}

pub fn sqrt_price_from_q64(sqrt_price_q64: Decimal) -> Decimal {
    sqrt_price_q64 / Decimal::from(2u128.pow(64))
}

pub fn target_sqrt_price_by_slippage(
    current_sqrt_price: Decimal,
    slippage: Decimal,
    zero_to_one: bool,
) -> Result<Decimal> {
    if zero_to_one {
        // price goes down
        return (current_sqrt_price.powu(2) * (Decimal::ONE - slippage))
            .sqrt()
            .ok_or_else(|| anyhow!("Failed to calculate target sqrt price"));
    }
    // price goes up
    (current_sqrt_price.powu(2) * (Decimal::ONE + slippage))
        .sqrt()
        .ok_or_else(|| anyhow!("Failed to calculate target sqrt price"))
}

pub fn delta_amount_from_sqrt_price(
    current_sqrt_price: Decimal,
    target_sqrt_price: Decimal,
    liquidity: Decimal,
) -> Result<(Decimal, Decimal)> {
    if current_sqrt_price.is_zero() || target_sqrt_price.is_zero() || liquidity.is_zero() {
        return Err(anyhow!(
            "Current or target sqrt price or liquidity cannot be zero"
        ));
    }

    // formula:
    // delta_x = (target_sqrt_price^-1 - current_sqrt_price^-1) * liquidity
    // delta_y = (target_sqrt_price - current_sqrt_price) * liquidity
    let delta_x = (target_sqrt_price.powi(-1) - current_sqrt_price.powi(-1)) * liquidity;
    let delta_y = (target_sqrt_price - current_sqrt_price) * liquidity;

    Ok((delta_x, delta_y))
}

pub fn target_sqrt_price_given_amount_in(
    current_sqrt_price: Decimal,
    amount_in: Decimal,
    liquidity: Decimal,
    zero_to_one: bool,
) -> Result<Decimal> {
    if current_sqrt_price.is_zero() || liquidity.is_zero() {
        return Err(anyhow!("Current sqrt price or liquidity cannot be zero"));
    }

    // if zero_to_one:
    // true: amount_in is delta_x
    // false: amount_in is delta_y
    let target_sqrt_price = if zero_to_one {
        // price goes down
        (current_sqrt_price.powi(-1) + amount_in / liquidity).powi(-1)
    } else {
        // price goes up
        current_sqrt_price + amount_in / liquidity
    };

    Ok(target_sqrt_price)
}

pub fn amount_out_given_target_sqrt_price(
    current_sqrt_price: Decimal,
    target_sqrt_price: Decimal,
    liquidity: Decimal,
    zero_to_one: bool,
) -> Result<Decimal> {
    if current_sqrt_price.is_zero() || liquidity.is_zero() {
        return Err(anyhow!("Current sqrt price or liquidity cannot be zero"));
    }

    // if zero_to_one:
    // true: amount_out is delta_y
    // false: amount_out is delta_x
    let amount_out = if zero_to_one {
        // price goes down
        (target_sqrt_price - current_sqrt_price) * liquidity
    } else {
        // price goes up
        (target_sqrt_price.powi(-1) - current_sqrt_price.powi(-1)) * liquidity
    };

    // amount_out calculated by maths formula is negative, so we need to negate it
    Ok(-amount_out)
}

// 0 is positive, 1 is negative
pub fn sign(tick: &str) -> Result<u8> {
    let tick_u32 = as_u32(tick)?;
    Ok(sign_u32(tick_u32))
}

pub fn sign_u32(tick: u32) -> u8 {
    (tick >> 31) as u8
}

pub fn as_u32(tick: &str) -> Result<u32> {
    tick.parse::<u32>()
        .map_err(|e| anyhow::anyhow!("Failed to parse tick '{}': {}", tick, e))
}

// XOR with 0xFFFFFFFF to negate a u32
pub fn u32_neg(tick: u32) -> Result<u32> {
    Ok(tick ^ 4294967295)
}

// The tick boundary is [-443636, 443636]
pub fn tick_bound() -> i32 {
    443636
}
