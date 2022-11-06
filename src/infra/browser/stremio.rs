use anyhow::Result;
use thirtyfour::WebDriver;
use tracing::info;

use crate::utils::env_key;

/// Tells ffmpeg to stream the stremio video as mp4
/// because the browser video player does not understand the .mkv format
/// which is the format used for stremio videos.
#[tracing::instrument(name = "stremio::open_stream_in_ffmpeg", skip_all, fields(url = %url))]
pub async fn open_stream_in_ffmpeg(driver: &WebDriver, url: &str) -> Result<tokio::process::Child> {
  kill_ffmpeg().await?;

  let path = format!(
    "http://localhost:{}/static/index.html?is_stremio_video=1",
    env_key("VIDEO_STREAM_API_PORT")?
  );

  info!("spawning ffmpeg process");
  let process = tokio::process::Command::new("ffmpeg")
    .args([
      "-i",
      // TODO: is it a problem to pass anything to the ffmpeg command?
      url,
      "-listen",
      "1",
      "-preset",
      "fast",
      "-f",
      "mp4",
      "-crf",
      "20",
      "-movflags",
      "frag_keyframe+empty_moov",
      // Video will be streamed as mp4 on this endpoint.
      "http://localhost:3001/video_stream",
    ])
    // Execute the command as a child process
    // so the bot does not block until the process is done executing.
    .spawn()?;

  info!("navigating to path. path={path}");
  driver.goto(path).await?;

  Ok(process)
}

/// Kill the ffmpeg process if it is running.
#[tracing::instrument(name = "stremio::kill_ffmpeg", skip_all)]
pub async fn kill_ffmpeg() -> Result<std::process::Output, std::io::Error> {
  tokio::process::Command::new("pkill")
    .arg("ffmpeg")
    .output()
    .await
}
