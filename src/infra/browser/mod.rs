use anyhow::Result;
use async_trait::async_trait;
use enigo::{Enigo, Key, KeyboardControllable};
use lazy_static::lazy_static;
use regex::Regex;
use serenity::model::prelude::{ChannelId, Message};
use std::time::Duration;
use thirtyfour::{
  prelude::{ElementQueryable, ScriptRet, WebDriverResult},
  By, DesiredCapabilities, WebDriver, WindowHandle,
};
use tokio::sync::{Mutex, MutexGuard};
use tracing::info;

use crate::{contracts, utils::env_key};

/// NOTE: For selenium 3.x, use "http://localhost:4444/wd/hub/session".
const SELENIUM_ENDPOINT: &str = "http://localhost:4444";
const WINDOW_WIDTH: i64 = 1920;
const WINDOW_HEIGHT: i64 = 1080;

lazy_static! {
  static ref GET_VIDEO_ID_FROM_YOUTUBE_URL_REGEX: Regex = Regex::new(r#"v=([\w\d_]+)"#).unwrap();
}

pub struct Browser {
  inner: Mutex<Inner>,
}

struct Inner {
  /// It is Some when the browser is open.
  driver: Option<WebDriver>,
  /// It is Some after the discord window is opened..
  discord_window: Option<WindowHandle>,
  /// It is Some after at least one video starts being played.
  video_tab: Option<WindowHandle>,
}

impl Browser {
  pub fn new() -> Self {
    Self {
      inner: Mutex::new(Inner {
        driver: None,
        discord_window: None,
        video_tab: None,
      }),
    }
  }

  #[tracing::instrument(name = "Browser::open_browser", skip_all)]
  async fn open_browser(&self) -> Result<WebDriver> {
    let mut caps = DesiredCapabilities::chrome();
    // Use google chrome instead of chromium because
    // the bot gets stuck on RTC connecting after joining a voice channel on chromium.
    caps.set_binary("/usr/bin/google-chrome")?;
    caps.add_chrome_arg("--autoplay-policy=no-user-gesture-required")?;
    caps.add_chrome_arg(&format!("--window-size={WINDOW_WIDTH},{WINDOW_HEIGHT}"))?;

    caps.add_chrome_option(
      "prefs",
      serde_json::json!( {
        "profile": {
          "content_settings": {
            "exceptions": {
              "media_stream_camera": {
                "https://*,*": {
                  "setting": 1
                }
              },
              "media_stream_mic": {
                "https://*,*": {
                  "setting": 1
                }
              }
            }
          }
        }
      }),
    )?;

    let driver = WebDriver::new(SELENIUM_ENDPOINT, caps).await?;

    Ok(driver)
  }

  #[tracing::instrument(name = "Browser::init_and_get_driver", skip_all)]
  async fn init_and_get_driver(&self) -> Result<MutexGuard<Inner>> {
    let mut inner = self.inner.lock().await;

    if inner.driver.is_none() {
      inner.driver = Some(self.open_browser().await?);
    }

    Ok(inner)
  }
}

#[tracing::instrument(name = "screen_share_video_tab_number_1", skip_all)]
async fn screen_share_video_tab_number_1(driver: &WebDriver) -> WebDriverResult<()> {
  open_discord_screen_share_screen_selection(driver).await?;

  tokio::time::sleep(Duration::from_millis(200)).await;

  let mut enigo = Enigo::new();

  // Select the tab that's playing the video as the screen to be shared.
  // Note that we use only keyboard keys because we cannot click on the popup using selenium.
  enigo.key_click(Key::Tab);
  enigo.key_click(Key::RightArrow);
  enigo.key_click(Key::RightArrow);
  enigo.key_click(Key::Tab);
  enigo.key_click(Key::DownArrow);
  enigo.key_click(Key::DownArrow);
  enigo.key_click(Key::Return);

  Ok(())
}

#[tracing::instrument(name = "browser::click_on_change_windows", skip_all)]
async fn click_on_change_windows(driver: &WebDriver) -> WebDriverResult<()> {
  driver
    .query(By::Id("manage-streams-change-windows"))
    .first()
    .await?
    .click()
    .await
}

#[tracing::instrument(name = "browser::open_discord_screen_share_screen_selection", skip_all)]
async fn open_discord_screen_share_screen_selection(driver: &WebDriver) -> WebDriverResult<()> {
  driver
    .query(By::Css(r#"button[aria-label="Share Your Screen"]"#))
    .first()
    .await?
    .click()
    .await

  // TODO: If we are already sharing the screen, click on Change Windows.
}

/// Kill the ffmpeg process if it is running.
#[tracing::instrument(name = "kill_ffmpeg", skip_all)]
async fn kill_ffmpeg() -> Result<std::process::Output, std::io::Error> {
  tokio::process::Command::new("pkill")
    .arg("ffmpeg")
    .output()
    .await
}

/// Tells ffmpeg to stream the stremio video as mp4
/// because the browser video player does not understand the .mkv format
/// which is the format used for stremio videos.
#[tracing::instrument(name = "open_stremio_stream_in_ffmpeg", skip_all, fields(url = %url))]
fn open_stremio_stream_in_ffmpeg(url: &str) -> Result<tokio::process::Child, std::io::Error> {
  tokio::process::Command::new("ffmpeg")
    .args(&[
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
    .spawn()
}

#[async_trait]
impl contracts::browser::Browser for Browser {
  #[tracing::instrument(name = "Browser::play_video_on_discord", skip_all, fields(url = %url))]
  async fn play_video_on_discord(&self, msg: &Message, url: &str) -> Result<()> {
    let mut inner = self.init_and_get_driver().await?;
    let driver = inner.driver.clone().unwrap();

    let server_url = format!(
      "https://discord.com/channels/{}/{}",
      msg.guild_id.expect("guild id should exist"),
      msg.channel_id
    );
    if inner.discord_window.is_none() {
      info!("navigating to discord page");
      driver.goto("https://discord.com").await?;

      tokio::time::sleep(Duration::from_millis(200)).await;

      login(&driver, env_key("DISCORD_SELF_BOT_TOKEN")?).await?;

      tokio::time::sleep(Duration::from_secs(3)).await;

      driver.refresh().await?;

      tokio::time::sleep(Duration::from_millis(200)).await;

      driver.goto(server_url).await?;

      tokio::time::sleep(Duration::from_millis(200)).await;

      if join_voice_channel(&driver, msg.channel_id).await.is_err() {
        tokio::time::sleep(Duration::from_secs(1)).await;
        join_voice_channel(&driver, msg.channel_id).await?;
      }

      inner.discord_window = Some(driver.window().await?);
    }

    tokio::time::sleep(Duration::from_millis(200)).await;

    kill_ffmpeg().await?;

    let path = if is_stremio_stream_link(url) {
      open_stremio_stream_in_ffmpeg(url)?;

      format!(
        "http://localhost:{}/static/index.html?is_stremio_video=1",
        env_key("VIDEO_STREAM_API_PORT")?
      )
    } else {
      format!(
        "http://localhost:{}/static/index.html?youtube_video_id={}",
        env_key("VIDEO_STREAM_API_PORT")?,
        get_video_id_from_youtube_url(url)?
      )
    };

    // If it is a new video being played after the previous one is done playing.
    if let Some(current_video_tab) = inner.video_tab.clone() {
      // Open the file that contains the video in the same tab that was being
      // used to play the previous video.
      info!("switching to video tab");
      driver.switch_to_window(current_video_tab).await?;
      info!("going to path. path={path}");
      driver.goto(path).await?;
    } else {
      // It is the first video being played by the bot so there's only two tabs:
      // The discord tab and the new video tab.

      info!("opening video in a new tab. path={path}");
      let new_video_tab = driver.new_tab().await?;
      inner.video_tab = Some(new_video_tab.clone());
      driver.switch_to_window(new_video_tab).await?;
      driver.goto(path).await?;

      info!("screen sharing video tab number 1");
      // SAFETY: initialized above.
      driver
        .switch_to_window(inner.discord_window.clone().unwrap())
        .await?;
      screen_share_video_tab_number_1(&driver).await?;
    }

    Ok(())
  }

  async fn is_video_playing(&self) -> Result<bool> {
    let is_video_playing = {
      let inner = self.inner.lock().await;

      let driver = match inner.driver.as_ref() {
        None => return Ok(false),
        Some(driver) => driver,
      };

      let video_tab = match inner.video_tab.clone() {
        None => return Ok(false),
        Some(window) => window,
      };

      let current_window = driver.window().await?;
      if current_window != video_tab {
        driver.switch_to_window(video_tab).await?;
      }

      // The player is added to the window in the html file.
      let ret = driver
        .execute(
          r#"
          if (window.player) {
            const UNSTARTED = -1
            const ENDED = 0
            const PLAYING = 1
            const PAUSED = 2
            const BUFFERING = 3
            const VIDEO_CUED = 5
            const playerState = window.player.getPlayerState()
            return [PLAYING, PAUSED, BUFFERING].includes(playerState) 
          }

          const stremioVideo = document.getElementById("stremio-stream-video")
          if (stremioVideo) {
            return stremioVideo.ended
          }

          return false
      "#,
          vec![],
        )
        .await?;

      ret.json().clone()
    };

    let playing = is_video_playing.as_bool().unwrap_or(false);

    Ok(playing)
  }

  #[tracing::instrument(name = "Browser::stop_current_video", skip_all)]
  async fn stop_current_video(&self) -> Result<()> {
    todo!()
  }
}

#[tracing::instrument(name = "browser::open_server", skip_all)]
async fn login(driver: &WebDriver, token: String) -> WebDriverResult<ScriptRet> {
  driver
  .execute(
    r#"
    function login(token) {
      setInterval(() => {
        document.body.appendChild(document.createElement `iframe`).contentWindow.localStorage.token = `"${token}"`
      }, 50);
    }

    login(arguments[0])
    "#,
    vec![serde_json::Value::String(token)],
  )
  .await
}

#[tracing::instrument(name = "browser::join_voice_channel", skip_all, fields(channel_id = %channel_id))]
async fn join_voice_channel(driver: &WebDriver, channel_id: ChannelId) -> WebDriverResult<()> {
  let selector = format!("a[data-list-item-id='channels___{channel_id}']");

  driver
    .query(By::Css(&selector))
    .first()
    .await?
    .click()
    .await
}

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
enum YoutubeUrlError {
  #[error("the url is not supported: {0}")]
  UnsupportedUrl(String),
  #[error("it was not possible to get the video id from the youtube url")]
  UnableToGetVideoId(String),
}

#[tracing::instrument(name = "browser::is_stremio_stream_link", skip_all, fields(url = %url, is_stremio_link))]
fn is_stremio_stream_link(url: &str) -> bool {
  // A stremio stream url will look like this: http://127.0.0.1:11470/9d6bc3eab9687dcfe75b2933e7b46872726580aa/1
  let is_stremio_link = url.starts_with("http://127.0.0.1:11470");

  tracing::Span::current().record("is_stremio_link", is_stremio_link);

  is_stremio_link
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
