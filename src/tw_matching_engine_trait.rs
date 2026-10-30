use async_trait::async_trait;
use crate::tw_main::{BuyRequest, SellRequest};

#[async_trait]
pub trait TwnEngine: Send + Sync {
    async fn buy(&self, req: BuyRequest);
    async fn sell(&self, req: SellRequest);
    async fn allocation(&self, username: &str) -> Option<u64>;
}
