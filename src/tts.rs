use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::{error, info};

use crate::contracts;

#[derive(Debug, Serialize)]
struct CreateSoundRequest {
  pub data: CreateSoundRequestData,
  pub engine: String,
}

#[derive(Debug, Serialize)]
struct CreateSoundRequestData {
  pub text: String,
  pub voice: String,
}

#[derive(Debug, Deserialize)]
struct CreateSoundResponse {
  pub id: String,
}

#[derive(Debug, Deserialize)]
struct GetSoundLocationResponse {
  pub location: String,
}

pub struct Tts {
  client: reqwest::Client,
}

impl Tts {
  pub fn new() -> Self {
    Self {
      client: reqwest::Client::new(),
    }
  }
}

#[async_trait]
impl contracts::TextToSpeech for Tts {
  /// Creates a mp3 file containing `text` and returns its url.
  #[tracing::instrument(skip_all)]
  async fn create_audio(&self, text: String) -> Result<String> {
    let body = CreateSoundRequest {
      engine: String::from("google"),
      data: CreateSoundRequestData {
        text,
        voice: String::from("pt-BR"),
      },
    };

    let response = self
      .client
      .post("https://api.soundoftext.com/sounds")
      .header("Host", "api.soundoftext.com")
      .header("Referer", "https://soundoftext.com/")
      .header("Content-Type", "application/json")
      .header("Origin", "https://soundoftext.com")
      .json(&body)
      .send()
      .await
      .with_context(|| format!("request_body={:?}", &body))?
      .json::<CreateSoundResponse>()
      .await
      .with_context(|| format!("request_body={:?}", &body))?;

    info!("created audio file. response={:?}", &response);

    let response = reqwest::Client::new()
      .get(format!(
        "https://api.soundoftext.com/sounds/{}",
        response.id
      ))
      .header("Host", "api.soundoftext.com")
      .header("Referer", "https://soundoftext.com/")
      .header("Content-Type", "application/json")
      .header("Origin", "https://soundoftext.com")
      .send()
      .await?;

    let response_body_text = response.text().await?;

    match serde_json::from_str::<GetSoundLocationResponse>(&response_body_text) {
      Err(err) => {
        let error = Err(anyhow::anyhow!(
          "unexpected tts response. request_body={:?}, response={:?} error={:?}",
          &body,
          response_body_text,
          err
        ));
        error!("error={:?}", error);
        error
      }
      Ok(data) => {
        info!("requested audio file location. response_body={:?}", &data);

        Ok(data.location)
      }
    }
  }
}
