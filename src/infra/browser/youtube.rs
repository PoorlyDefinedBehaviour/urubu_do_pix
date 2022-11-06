use crate::utils::env_key;
use anyhow::Result;
use lazy_static::lazy_static;
use regex::Regex;
use thirtyfour::WebDriver;
use tracing::info;

lazy_static! {
  static ref GET_VIDEO_ID_FROM_YOUTUBE_URL_REGEX: Regex = Regex::new(r#"v=([\w\d_]+)"#).unwrap();
}

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
enum YoutubeUrlError {
  #[error("the url is not supported: {0}")]
  UnsupportedUrl(String),
  #[error("it was not possible to get the video id from the youtube url")]
  UnableToGetVideoId(String),
}

#[tracing::instrument(name = "youtube::open_video", skip_all, fields(url = %url))]
pub async fn open_video(driver: &WebDriver, url: &str) -> Result<()> {
  let path = format!(
    "http://localhost:{}/static/index.html?youtube_video_id={}",
    env_key("VIDEO_STREAM_API_PORT")?,
    get_video_id_from_youtube_url(url)?
  );

  info!("navigating to path. path={path}");
  driver.goto(path).await?;

  Ok(())
}

#[tracing::instrument(name = "browser::get_video_id_from_youtube_url", skip_all, fields(url = %url))]
fn get_video_id_from_youtube_url(url: &str) -> Result<String, YoutubeUrlError> {
  if !url.starts_with("https://www.youtube.com/watch") {
    return Err(YoutubeUrlError::UnsupportedUrl(url.to_owned()));
  }

  match GET_VIDEO_ID_FROM_YOUTUBE_URL_REGEX.captures(url) {
    None => Err(YoutubeUrlError::UnableToGetVideoId(url.to_owned())),
    Some(captures) => {
      let video_id = &captures[1];
      Ok(video_id.to_owned())
    }
  }
}

#[cfg(test)]
mod get_video_id_from_youtube_url_tests {
  use super::*;

  #[test]
  fn simple() {
    assert_eq!(
      get_video_id_from_youtube_url("https://www.youtube.com/watch?v=fy_SkwBOcXA"),
      Ok("fy_SkwBOcXA".to_owned())
    )
  }
}
