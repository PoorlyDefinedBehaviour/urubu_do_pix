use anyhow::Result;
use async_trait::async_trait;
use std::time::Duration;

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait Cache: Send + Sync {
  async fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>>;
  async fn put(&self, key: Vec<u8>, value: Vec<u8>, ttl: Duration) -> Result<()>;
}
