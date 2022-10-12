use anyhow::Result;
use async_trait::async_trait;

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait TextToSpeech: Send + Sync {
  async fn create_audio(&self, text: String) -> Result<Vec<String>>;
}
