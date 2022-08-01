use std::time::Duration;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use tracing::{error, info};

use crate::utils::env_key;

#[derive(Debug, Serialize)]
struct ChatBotRequest {
  pub text: String,
  pub temperature: f32,
  pub repetition_penalty: f32,
  pub top_p: u32,
  pub top_k: u32,
  pub response_length: u32,
}

#[derive(Debug, Deserialize)]
struct ChatBotResponse {
  pub data: String,
}

pub struct TextGenerator {}

impl TextGenerator {
  pub fn new() -> Self {
    Self {}
  }

  /// Generates text based on `Context`. If you want it to talk about soccer,
  /// pass a context that contains a conversation about soccer.
  #[tracing::instrument(skip_all)]
  pub async fn generate(&self, context: String) -> Result<String> {
    let response = reqwest::Client::new()
      .post("https://model-api-shdxwd54ta-nw.a.run.app/generate/gptj")
      .header("Host", "model-api-shdxwd54ta-nw.a.run.app")
      .header("Referer", "https://chai.ml/")
      .header("Content-Type", "application/json")
      .header("developer_uid", env_key("CHAIML_DEVELOPER_UUID")?)
      .header("developer_key", env_key("CHAIML_KEY")?)
      .header("Origin", "https://chai.ml")
      .json(&ChatBotRequest {
        text: context,
        temperature: 0.6,
        repetition_penalty: 1.1,
        top_p: 1,
        top_k: 40,
        response_length: 64,
      })
      .timeout(Duration::from_secs(5))
      .send()
      .await?;

    let response_body_text = response.text().await?;

    match serde_json::from_str::<ChatBotResponse>(&response_body_text) {
      Err(err) => {
        let error = Err(anyhow::anyhow!(
          "unexpected chat bot response. response={:?} error={:?}",
          response_body_text,
          err
        ));
        error!("error={:?}", error);
        error
      }
      Ok(body) => {
        info!("text generated. text={}", &body.data);

        Ok(body.data.replace("Eliza:", ""))
      }
    }
  }
}
