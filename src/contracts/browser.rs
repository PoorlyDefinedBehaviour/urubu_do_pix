use anyhow::Result;
use async_trait::async_trait;
use serenity::model::prelude::Message;

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait Browser: Send + Sync {
  /// Opens the browser and screen shares a video.
  async fn play_video_on_discord(&self, msg: &Message, url: &str) -> Result<()>;

  /// Returns true when a video is being played.
  async fn is_video_playing(&self) -> Result<bool>;

  /// Stops playing the current video, if there's one.
  async fn stop_current_video(&self) -> Result<()>;
}
