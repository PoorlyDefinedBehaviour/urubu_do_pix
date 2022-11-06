use std::time::Duration;

use anyhow::Result;
use enigo::{Enigo, KeyboardControllable};
use thirtyfour::WebDriver;

#[tracing::instrument(name = "twitch::open_live", skip_all, fields(
  stream_url = %stream_url
))]
pub async fn open_live(driver: &WebDriver, stream_url: &str) -> Result<()> {
  driver.goto(stream_url).await?;

  tokio::time::sleep(Duration::from_millis(200)).await;

  toggle_theatre_mode();

  Ok(())
}

fn toggle_theatre_mode() {
  let mut enigo = Enigo::new();
  enigo.key_sequence_parse("{+ALT}t{-ALT}");
}
