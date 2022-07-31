use anyhow::{Context, Result};
use serenity::model::channel::Message;
use tracing::error;

pub fn check_message(res: serenity::Result<Message>) {
  if let Err(why) = res {
    error!("Error sending message: {:?}", why)
  }
}

pub fn env_key(key: &str) -> Result<String> {
  std::env::var(key)
    .ok()
    .context(format!("missing env variable: {}", key))
}
