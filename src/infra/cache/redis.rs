use std::time::Duration;

use crate::contracts;
use anyhow::Result;
use async_trait::async_trait;
use redis::AsyncCommands;

#[derive(Debug)]
pub struct Config {
  pub host: String,
  pub port: u16,
  pub password: String,
}

pub struct RedisCache {
  client: redis::Client,
}

impl RedisCache {
  #[tracing::instrument(skip_all)]
  pub fn new(config: Config) -> Result<Self> {
    let client = redis::Client::open(redis::ConnectionInfo {
      addr: redis::ConnectionAddr::Tcp(config.host, config.port),
      redis: redis::RedisConnectionInfo {
        db: 0,
        username: None,
        password: Some(config.password),
      },
    })?;

    Ok(Self { client })
  }
}

#[async_trait]
impl contracts::Cache for RedisCache {
  #[tracing::instrument(skip_all, fields(key = %String::from_utf8_lossy(key)))]
  async fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>> {
    let value: Option<Vec<u8>> = self.client.get_async_connection().await?.get(key).await?;
    Ok(value)
  }

  #[tracing::instrument(skip_all, fields(key = %String::from_utf8_lossy(&key)))]
  async fn put(&self, key: Vec<u8>, value: Vec<u8>, ttl: Duration) -> Result<()> {
    self
      .client
      .get_async_connection()
      .await?
      .set_ex(key, value, ttl.as_secs() as usize)
      .await?;
    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use crate::contracts::Cache;

  use super::*;

  #[tokio::test]
  async fn basic() -> Result<(), Box<dyn std::error::Error>> {
    let redis = RedisCache::new(Config {
      host: String::from("127.0.0.1"),
      port: 6379,
      password: String::from("password"),
    })?;

    assert_eq!(None, redis.get(b"i_dont_exist").await?);

    let key = b"key".to_vec();
    let value = b"value".to_vec();

    redis
      .put(key.clone(), value.clone(), Duration::from_secs(60))
      .await?;

    let result = redis.get(&key).await?;

    assert_eq!(Some(value), result);

    Ok(())
  }
}
