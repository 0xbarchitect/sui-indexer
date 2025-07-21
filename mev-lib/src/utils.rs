pub mod ptb;
pub mod tick_math;

use crate::constant;
use db::repositories::{CoinRepository, PoolRepository};

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use bigdecimal::BigDecimal;
use chrono::{DateTime, NaiveDateTime, Utc};
use futures::stream::{self, StreamExt};
use regex::Regex;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    hash::{Hash, Hasher},
    str::FromStr,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};
use sui_sdk::{
    rpc_types::{Coin, SuiData, SuiObjectDataOptions},
    SuiClient,
};
use sui_types::{
    base_types::{ObjectID, SuiAddress},
    crypto::{EncodeDecodeBase64, SuiKeyPair},
    event::Event,
    sui_system_state::sui_system_state_inner_v1::SystemParametersV1,
    transaction::{Argument, ObjectArg},
};
use tokio::time::{Duration, Instant};
use tracing::{debug, error, info, trace, warn, Level};
use twox_hash::XxHash64;

pub fn load_keypair_from_priv_key(priv_key: &str) -> Result<SuiKeyPair> {
    let skp =
        SuiKeyPair::decode(priv_key).map_err(|e| anyhow!("Failed to decode keypair: {}", e))?;
    Ok(skp)
}

pub fn load_keypair_from_base64_key(base64_key: &str) -> Result<SuiKeyPair> {
    let skp = SuiKeyPair::decode_base64(base64_key)
        .map_err(|e| anyhow!("Failed to decode keypair from base64: {}", e))?;
    Ok(skp)
}

pub fn amount_to_mist(amount: f64, decimals: u8) -> u64 {
    (amount * 10f64.powi(decimals as i32)) as u64
}

pub fn mist_to_amount(mist_amount: u64, decimals: u8) -> f64 {
    mist_amount as f64 / (10u64.pow(decimals as u32) as f64)
}

/// Extracts the coin types from the pool type string using Regex
/// Returns a tuple containing the coin types.
pub fn get_coin_types_from_pool_type(pool_type: &str, exchange: &str) -> Result<Vec<String>> {
    let re = Regex::new(r"<(.*)>")?;
    let caps = re
        .captures(pool_type)
        .ok_or_else(|| anyhow!("Failed to capture coin types from pool type: {}", pool_type))?;

    let coins: Vec<String> = caps
        .get(1)
        .ok_or(anyhow!("Failed to get coin types from pool type"))?
        .as_str()
        .split(',')
        .map(|s| s.trim().to_string())
        .collect();

    if coins.len() < 2 {
        return Err(anyhow!(
            "Failed to extract coin types from pool type: {}",
            pool_type
        ));
    }

    match exchange {
        "cetus" | "obric" | "bluefin" | "momentum" | "flowx" | "bluemove" => Ok(coins),
        "turbos" => Ok(vec![coins[0].clone(), coins[1].clone()]),
        _ => Err(anyhow!("Upsupported exchange {}", exchange)),
    }
}

/// Extracts the pool type from a full pool type string.
/// E.g: 0xd1a3eab6e9659407cb2a5a529d13b4102e498619466fc2d01cb0a6547bbdb376::af_lp::AF_LP
/// from 0xefe170ec0be4d762196bedecd7a065816576198a6527c99282a2551aaa7da38c::pool::Pool<0xd1a3eab6e9659407cb2a5a529d13b4102e498619466fc2d01cb0a6547bbdb376::af_lp::AF_LP>
///
pub fn extract_pool_type(pool_type_full: &str, exchange: &str) -> Result<String> {
    match exchange {
        "aftermath" => {
            let re = Regex::new(r"<(0x[a-zA-Z0-9_]+::[a-zA-Z0-9_]+::[a-zA-Z0-9_]+)>")?;
            if let Some(captures) = re.captures(pool_type_full) {
                if let Some(pool_type) = captures.get(1) {
                    return Ok(pool_type.as_str().to_string());
                }
            }
            Err(anyhow!(
                "Failed to extract pool type from: {}",
                pool_type_full
            ))
        }
        "turbos" => {
            let re = Regex::new(r"<(.*)>")?;
            let caps = re.captures(pool_type_full).ok_or_else(|| {
                anyhow!(
                    "Failed to capture coin types from pool type: {}",
                    pool_type_full
                )
            })?;

            let coins: Vec<String> = caps
                .get(1)
                .ok_or(anyhow!("Failed to get coin types from pool type"))?
                .as_str()
                .split(',')
                .map(|s| s.trim().to_string())
                .collect();

            if coins.len() < 3 {
                return Err(anyhow!(
                    "Failed to extract coin types from pool type: {}",
                    pool_type_full
                ));
            }

            Ok(coins[2].clone())
        }
        _ => Err(anyhow!("Exchange: {} does not have pool type", exchange)),
    }
}

pub fn convert_log_level_to_tracing_level(log_level: &str) -> Level {
    match log_level.to_lowercase().as_str() {
        "error" => Level::ERROR,
        "warn" => Level::WARN,
        "info" => Level::INFO,
        "debug" => Level::DEBUG,
        "trace" => Level::TRACE,
        _ => Level::INFO, // Default to INFO if the log level is not recognized
    }
}

pub fn convert_number_vec_to_hex_string(numbers: &[u8]) -> String {
    let hex_string: String = numbers.iter().map(|num| format!("{:02x}", num)).collect();

    format!("0x{}", hex_string)
}

pub fn timestamp_to_naive_datetime(timestamp: u64) -> NaiveDateTime {
    // Unix timestamp is typically in seconds
    NaiveDateTime::from_timestamp_opt(timestamp as i64, 0).unwrap_or_default()
}

/// Format a coin type to standard format `0x<package>::<module>::<name>`
/// If `with_prefix` is true, it will include the `0x` prefix.
/// If the type is a SUI coin type, it will return a constant SUI value.
///
pub fn format_type_name(full_type_name: &str, with_prefix: bool) -> String {
    // check if the type is SUI coin type
    let re = Regex::new(r"^[0x]+(2::sui::SUI)$").unwrap();
    if re.is_match(full_type_name) {
        return constant::SUI_COIN.to_string();
    }

    // Create a regex to match the address part followed by module and name
    let re = Regex::new(r"^([0-9a-fA-F]+)(::.*$)").unwrap();

    // Apply the regex and format the result
    if let Some(captures) = re.captures(full_type_name) {
        let mut address = captures.get(1).unwrap().as_str().to_string();

        if address.len() > 64 {
            address = address.chars().take(64).collect();
        }

        if address.len() < 64 {
            let padding = "0".repeat(64 - address.len());
            // Pad the address to 64 characters with leading zeros
            address = format!("{}{}", padding, address);
        }

        let remainder = captures.get(2).unwrap().as_str();

        // Handle the case where address is empty (all zeros)
        let addr_formatted = if address.is_empty() {
            "0".to_string()
        } else {
            address
        };

        if with_prefix {
            format!("0x{}{}", addr_formatted, remainder)
        } else {
            format!("{}{}", addr_formatted, remainder)
        }
    } else {
        // Return unchanged if not matching expected pattern
        full_type_name.to_string()
    }
}

pub fn format_pyth_feed_id(feed_id: &str, with_prefix: bool) -> String {
    let re = Regex::new(r"^(0x)([0-9a-fA-F]+)").unwrap();

    if let Some(captures) = re.captures(feed_id) {
        let feed_id = captures.get(2).unwrap().as_str();
        if with_prefix {
            format!("0x{}", feed_id)
        } else {
            feed_id.to_string()
        }
    } else if with_prefix {
        format!("0x{}", feed_id)
    } else {
        feed_id.to_string()
    }
}

pub fn extract_event_type(event: &str) -> Result<String> {
    let re = Regex::new(r"([a-zA-Z0-9_:]+::[a-zA-Z0-9_]+::[a-zA-Z0-9_]+)").unwrap();

    if let Some(captures) = re.captures(event) {
        if let Some(event_type) = captures.get(1) {
            return Ok(event_type.as_str().to_string());
        }
    }
    Err(anyhow!("Failed to extract event type from: {}", event))
}

pub fn convert_q64_to_decimal_price(sqrt_price: &str) -> Result<Decimal> {
    let sqrt_price =
        Decimal::from_str(sqrt_price).map_err(|e| anyhow!("Failed to parse sqrt_price: {}", e))?;

    if sqrt_price.is_zero() {
        return Err(anyhow!("Sqrt price cannot be zero"));
    }

    let denominator = Decimal::from(2u128.pow(64));
    let sqrt_decimal = sqrt_price / denominator;
    let price = sqrt_decimal * sqrt_decimal;
    Ok(price)
}

pub fn sui_from_mist(mist: Decimal, decimals: usize) -> Decimal {
    let factor = Decimal::from(10u128.pow(decimals as u32));
    mist / factor
}

pub fn mist_from_sui(sui: Decimal, decimals: usize) -> Decimal {
    let factor = Decimal::from(10u128.pow(decimals as u32));
    sui * factor
}

pub fn generate_borrower_id(platform: &str, address: &str) -> u64 {
    let mut hasher = XxHash64::default();
    platform.hash(&mut hasher);
    address.hash(&mut hasher);
    hasher.finish()
}

pub fn generate_market_id(platform: &str, coin_type: &str) -> u64 {
    let mut hasher = XxHash64::default();
    platform.hash(&mut hasher);
    coin_type.hash(&mut hasher);
    hasher.finish()
}

pub fn net_value_given_fee_rate(gross_value: Decimal, fee_rate: Decimal) -> Result<Decimal> {
    if fee_rate >= Decimal::ONE || fee_rate < Decimal::ZERO {
        return Err(anyhow!("Invalid fee rate: must be between 0 and 1"));
    }

    Ok(gross_value * (Decimal::ONE - fee_rate))
}

pub fn gross_value_given_fee_rate(net_value: Decimal, fee_rate: Decimal) -> Result<Decimal> {
    if fee_rate >= Decimal::ONE || fee_rate < Decimal::ZERO {
        return Err(anyhow!("Invalid fee rate: must be between 0 and 1"));
    }

    Ok(net_value / (Decimal::ONE - fee_rate))
}

pub fn threshold_value_given_slippage(
    expected_value: Decimal,
    slippage: Decimal,
) -> Result<Decimal> {
    if slippage >= Decimal::ONE || slippage < Decimal::ZERO {
        return Err(anyhow!("Invalid slippage: must be between 0 and 1"));
    }

    Ok(expected_value * (Decimal::ONE - slippage))
}

pub fn deserialize_tick_index<'de, D>(deserializer: D) -> Result<u32, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde_json::Value;
    let v = Value::deserialize(deserializer)?;

    if let Some(bits) = v.get("bits").and_then(|b| b.as_u64()) {
        return Ok(bits as u32);
    }
    if let Some(fields) = v.get("fields") {
        if let Some(bits) = fields.get("bits").and_then(|b| b.as_u64()) {
            return Ok(bits as u32);
        }
    }
    Err(serde::de::Error::custom("bits not found"))
}

/// Returns the current timestamp in milliseconds since the Unix epoch.
pub fn get_current_timestamp_ms() -> u64 {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards");
    now.as_millis() as u64
}

pub fn get_current_timestamp_secs() -> u64 {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards");
    now.as_secs()
}

pub fn lagging_timestamp_ms(latest_timestamp_ms: u64) -> u64 {
    if latest_timestamp_ms == 0 {
        return 0;
    }
    let current_timestamp = get_current_timestamp_ms();
    if current_timestamp < latest_timestamp_ms {
        return 0;
    }
    current_timestamp - latest_timestamp_ms
}

pub fn lagging_timestamp_secs(latest_timestamp_secs: u64) -> u64 {
    if latest_timestamp_secs == 0 {
        return 0;
    }
    let current_timestamp = get_current_timestamp_secs();
    if current_timestamp < latest_timestamp_secs {
        return 0;
    }
    current_timestamp - latest_timestamp_secs
}

pub fn convert_bigdecimal_to_decimal(big_decimal: &BigDecimal, scale: i64) -> Result<Decimal> {
    let rounded = big_decimal.with_scale(scale);
    Decimal::from_str(&rounded.to_string()).map_err(|e| {
        anyhow!(
            "Failed to convert BigDecimal to Decimal with scale {}: {}",
            scale,
            e
        )
    })
}

pub fn bigdecimal_for_decimals(decimals: u8) -> BigDecimal {
    let scale = 10u32.pow(decimals as u32);
    BigDecimal::from(scale)
}

pub fn format_coin_type_onchain(coin_type: &str) -> Result<String> {
    let re = Regex::new(r"^0x([a-zA-Z0-9_]+)::([a-zA-Z0-9_]+)::([a-zA-Z0-9_]+)$").unwrap();
    if let Some(captures) = re.captures(coin_type) {
        let mut part1 = captures.get(1).unwrap().as_str().to_string();
        if part1.len() > 64 {
            return Err(anyhow!("Invalid coin type: {}", coin_type));
        }

        if part1.len() < 64 {
            let padding = "0".repeat(64 - part1.len());
            part1 = format!("{}{}", &padding, &part1);
        }

        Ok(format!(
            "{}::{}::{}",
            part1,
            captures.get(2).unwrap().as_str(),
            captures.get(3).unwrap().as_str()
        ))
    } else {
        Err(anyhow!("Invalid coin type format: {}", coin_type))
    }
}
