use db::{
    models::{self, metric},
    repositories::MetricRepository,
};

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};
use std::fmt::{self, Display, Formatter};
use std::{
    hash::{Hash, Hasher},
    str::FromStr,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Coin {
    pub coin_type: String,
    pub decimals: u8,
    pub name: Option<String>,
    pub symbol: Option<String>,
    pub pyth_feed_id: Option<String>,
    pub pyth_info_object_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pool {
    pub exchange: String,
    pub pool_id: String,
    pub pool_type: Option<String>,
    pub coins: Vec<Coin>,
    pub coin_amounts: Option<Vec<String>>,
    pub weights: Option<Vec<String>>,
    pub tick_spacing: Option<i32>,
    pub current_tick_index: Option<i32>,
    pub current_sqrt_price: Option<String>,
    pub liquidity: Option<String>,
    pub fee_rate: Option<i32>,
    pub is_pause: Option<bool>,
    pub fees_swap_in: Option<Vec<String>>,
    pub fees_swap_out: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlashloanPool {
    pub exchange: String,
    pub pool_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PythPrice {
    pub feed_id: String,
    pub spot_price: String,
    pub ema_price: String,
    pub decimals: u8,
    pub latest_updated_timestamp: u64,
    pub vaa: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Borrower {
    pub platform: String,
    pub borrower: String,
    pub obligation_id: Option<String>,
    pub status: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BorrowerAsset {
    pub coin_type: String,
    pub asset_id: Option<i32>,
    pub pyth_info_object_id: String,
    pub navi_feed_id: Option<String>,
    pub vaa: Option<String>,
}

impl PartialEq for BorrowerAsset {
    fn eq(&self, other: &Self) -> bool {
        self.coin_type == other.coin_type
    }
}

impl Eq for BorrowerAsset {}

impl Hash for BorrowerAsset {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.coin_type.hash(state);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserBorrow {
    pub platform: String,
    pub borrower: String,
    pub obligation_id: Option<String>,
    pub coin_type: String,
    pub amount: String,
    pub debt_borrow_index: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserDeposit {
    pub platform: String,
    pub borrower: String,
    pub obligation_id: Option<String>,
    pub coin_type: String,
    pub amount: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metric {
    pub latest_seq_number: i32,
    pub total_checkpoints: i32,
    pub total_processed_checkpoints: i32,
    pub max_processing_time: f32,
    pub min_processing_time: f32,
    pub avg_processing_time: f32,
    pub max_lagging: f32,
    pub min_lagging: f32,
    pub avg_lagging: f32,
}

impl From<Metric> for db::models::metric::NewMetric {
    fn from(metric: Metric) -> Self {
        db::models::metric::NewMetric {
            latest_seq_number: metric.latest_seq_number,
            total_checkpoints: metric.total_checkpoints,
            total_processed_checkpoints: metric.total_processed_checkpoints,
            max_processing_time: metric.max_processing_time,
            min_processing_time: metric.min_processing_time,
            avg_processing_time: metric.avg_processing_time,
            max_lagging: metric.max_lagging,
            min_lagging: metric.min_lagging,
            avg_lagging: metric.avg_lagging,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PythPriceIdentifier {
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct U256 {
    pub v: [u64; 4],
}

impl Display for U256 {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let value = self.v[0] as u128 + ((self.v[1] as u128) << 64);
        write!(f, "{}", value)
    }
}

impl FromStr for U256 {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let value = s.parse::<u128>()?; // hoặc parse thành BigUint nếu cần lớn
        Ok(U256 {
            v: [
                (value & ((1u128 << 64) - 1)) as u64,
                (value >> 64) as u64,
                0,
                0,
            ],
        })
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TypeName {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixedPoint32 {
    pub value: u64,
}

impl Display for FixedPoint32 {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.value)
    }
}

impl FromStr for FixedPoint32 {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let value = s.parse::<u64>()?;
        Ok(FixedPoint32 { value })
    }
}

#[serde_as]
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FixedPoint32Json {
    #[serde_as(as = "DisplayFromStr")]
    pub value: u64,
}

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnchainDecimal {
    #[serde_as(as = "DisplayFromStr")]
    pub value: U256,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct I32 {
    pub bits: u32,
}

#[serde_as]
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct I128Json {
    #[serde_as(as = "DisplayFromStr")]
    pub bits: u128,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct I128 {
    pub bits: u128,
}

impl I128 {
    pub fn from_json(json: &I128Json) -> Self {
        I128 { bits: json.bits }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ObjectIDWrapper {
    pub id: String,
}
