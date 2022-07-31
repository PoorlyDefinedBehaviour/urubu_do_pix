use anyhow::Result;
use serde::{Deserialize, Serialize};
use tracing::info;

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

  /// Creates a mp3 file containing `text` and returns its url.
  #[tracing::instrument(skip_all)]
  pub async fn create_audio(&self, text: String) -> Result<String> {
    let response = self
      .client
      .post("https://api.soundoftext.com/sounds")
      .header("Host", "api.soundoftext.com")
      .header("Referer", "https://soundoftext.com/")
      .header("Content-Type", "application/json")
      .header("Origin", "https://soundoftext.com")
      .json(&CreateSoundRequest {
        engine: String::from("google"),
        data: CreateSoundRequestData {
          text,
          voice: String::from("pt-BR"),
        },
      })
      .send()
      .await?
      .json::<CreateSoundResponse>()
      .await?;

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
      .await?
      .json::<GetSoundLocationResponse>()
      .await?;

    info!("requested audio file location. response={:?}", &response);

    Ok(response.location)
  }
}
