use crate::contracts;
use anyhow::Result;
use serenity::{model::prelude::Message, prelude::Context};
use std::{
  collections::VecDeque,
  sync::{Arc, Weak},
  time::Duration,
};
use tokio::sync::Mutex;
use tracing::{info, warn};

pub struct Video {
  browser: Arc<dyn contracts::browser::Browser>,
  /// Queue of videos to be streamed.
  queue: Mutex<VecDeque<VideoRequest>>,
}

struct VideoRequest {
  msg: Message,
  url: String,
}

impl Video {
  #[tracing::instrument(name = "Video::new", skip_all)]
  pub fn new(browser: Arc<dyn contracts::browser::Browser>) -> Arc<Self> {
    let video = Arc::new(Self {
      browser,
      queue: Mutex::new(VecDeque::new()),
    });

    tokio::spawn(check_if_theres_a_video_to_play(Arc::downgrade(&video)));

    video
  }

  #[tracing::instrument(name = "Video::play", skip_all, fields(url = %url))]
  pub async fn play(&self, ctx: &Context, msg: &Message, url: &str) -> Result<()> {
    let mut queue = self.queue.lock().await;

    queue.push_back(VideoRequest {
      msg: msg.clone(),
      url: url.to_owned(),
    });

    msg.reply(ctx, "Added to queue").await?;

    Ok(())
  }

  #[tracing::instrument(name = "Video::skip_current_video", skip_all)]
  pub async fn skip_current_video(&self, _ctx: &Context, _msg: &Message) -> Result<()> {
    // Should work with normal videos and playlists.
    // Can we just seek to the last second in the video and let the youtube frame api handle it?
    // player.seekTo(seconds:Number, allowSeekAhead:Boolean):Void
    // self.browser.skip()
    Ok(())
  }

  #[tracing::instrument(name = "Video::stop", skip_all)]
  pub async fn stop(&self) -> Result<()> {
    self.browser.stop_current_video().await?;
    Ok(())
  }

  async fn check_if_theres_a_video_to_play(&self) -> Result<()> {
    if !self.browser.is_video_playing().await? {
      let mut queue = self.queue.lock().await;
      if let Some(video_request) = queue.pop_front() {
        self
          .browser
          .play_video_on_discord(&video_request.msg, &video_request.url)
          .await?;
      }
    }

    Ok(())
  }
}

#[tracing::instrument(name = "video::check_if_theres_a_video_to_play", skip_all)]
async fn check_if_theres_a_video_to_play(video: Weak<Video>) {
  loop {
    match video.upgrade() {
      None => {
        info!("Video has been dropped, quitting");
        break;
      }
      Some(video) => {
        if let Err(err) = video.check_if_theres_a_video_to_play().await {
          warn!(
            "unable to check if there's a video to play. error={:?}",
            err
          );
        }
      }
    }

    tokio::time::sleep(Duration::from_secs(1)).await;
  }
}
