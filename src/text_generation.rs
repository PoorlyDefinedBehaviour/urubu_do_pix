use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{sync::Arc, time::Duration};
use tracing::{error, info};

use crate::contracts::{self, PostOptions};

#[derive(Debug, Serialize)]
struct ChatBotRequest<'a> {
  pub text: &'a str,
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

#[derive(Debug)]
pub struct Config {
  pub chaiml_developer_uuid: String,
  pub chaiml_key: String,
}

pub struct TextGenerator {
  config: Config,
  http_client: Arc<dyn contracts::HttpClient>,
}

impl TextGenerator {
  pub fn new(config: Config, http_client: Arc<dyn contracts::HttpClient>) -> Self {
    Self {
      http_client,
      config,
    }
  }

  /// Generates text based on `Context`. If you want it to talk about soccer,
  /// pass a context that contains a conversation about soccer.
  #[tracing::instrument(skip_all)]
  pub async fn generate(&self, context: &str) -> Result<String> {
    let body = ChatBotRequest {
      text: context,
      temperature: 0.6,
      repetition_penalty: 1.1,
      top_p: 1,
      top_k: 40,
      response_length: 64,
    };

    let response = self
      .http_client
      .post(
        "https://model-api-shdxwd54ta-nw.a.run.app/generate/gptj",
        Some(PostOptions {
          headers: Some(vec![
            (
              "Host".to_string(),
              "model-api-shdxwd54ta-nw.a.run.app".to_string(),
            ),
            ("Referer".to_string(), "https://chai.ml/".to_string()),
            ("Content-Type".to_string(), "application/json".to_string()),
            (
              "developer_uid".to_string(),
              self.config.chaiml_developer_uuid.clone(),
            ),
            ("developer_key".to_string(), self.config.chaiml_key.clone()),
            ("Origin".to_string(), "https://chai.ml".to_string()),
          ]),
          timeout: Some(Duration::from_secs(30)),
        }),
      )
      .await?;

    match serde_json::from_slice::<ChatBotResponse>(&response.body) {
      Err(err) => {
        let error = Err(anyhow::anyhow!(
          "unexpected chat bot response. request_body={:?}, response={:?} error={:?}",
          &body,
          String::from_utf8_lossy(&response.body),
          err
        ));
        error!("error={:?}", error);
        error
      }
      Ok(body) => {
        info!("text generated. text={}", &body.data);

        for target in ["Eliza: ", "Eliza:", "Me: ", "Me:"] {
          if body.data.starts_with(target) {
            return Ok(body.data.replace(target, ""));
          }
        }

        Ok(body.data)
      }
    }
  }
}

#[cfg(test)]
mod generate_tests {
  use bytes::Bytes;

  use crate::contracts::{MockHttpClient, PostResponse};

  use super::*;

  #[tokio::test]
  async fn removes_unnecesary_prefix_from_generated_text() -> Result<(), Box<dyn std::error::Error>>
  {
    let tests = vec![
      (
        "Eliza: something something, blah blah",
        "something something, blah blah",
      ),
      (
        "Eliza:something something, blah blah",
        "something something, blah blah",
      ),
      (
        "Me: something something, blah blah",
        "something something, blah blah",
      ),
      (
        "Me:something something, blah blah",
        "something something, blah blah",
      ),
    ];

    for (input, expected) in tests.into_iter() {
      let mut http_client = MockHttpClient::new();

      http_client.expect_post().returning(move |_, _| {
        Ok(PostResponse {
          body: Bytes::from(serde_json::to_string(&serde_json::json!({
            "data": input
          }))?),
        })
      });

      let generator = TextGenerator::new(
        Config {
          chaiml_developer_uuid: "uuid".to_string(),
          chaiml_key: "key".to_string(),
        },
        Arc::new(http_client),
      );

      let generated_text = generator.generate("some context").await?;

      assert_eq!(expected, generated_text);
    }

    Ok(())
  }
}
