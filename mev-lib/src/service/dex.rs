pub mod aftermath;
pub mod bluefin;
pub mod bluemove;
pub mod cetus;
pub mod flowx;
pub mod momentum;
pub mod obric;
pub mod turbos;

use anyhow::{anyhow, Result};
use async_trait::async_trait;

#[async_trait]
pub trait DEXService {
    /// Fetches the pool data from the Sui client using the provided pool ID.
    async fn get_pool_data(&self, pool_id: &str) -> Result<crate::types::Pool>;
}
