use std::time::Duration;

use tracing::info;
use anyhow::Result;

pub struct Translation {
  client: reqwest::Client
}

impl Translation {
  pub fn new() -> Self {
    Self {
      client: reqwest::Client::new(),
    }
  }

  #[tracing::instrument(skip_all, fields(
    from_lang = %from_lang, 
    to_lang = %to_lang,
    text = %text
  ))]
  pub async fn translate(&self, text: &str, from_lang: &str, to_lang: &str) -> Result<String> {
    let response = self.client.get("https://translate.googleapis.com/translate_a/single?client=gtx")
      .query(&[("sl", from_lang),("tl" ,to_lang), ("dt","t"), ("q", text)])
      .timeout(Duration::from_secs(10))
      .send()
      .await?
      .json::<serde_json::Value>()
      .await?;

      match &response[0] {
        serde_json::Value::Array(translations) => {
          let mut phrases = Vec::with_capacity(translations.len());

          for translation in translations.iter() {
            phrases.push(match &translation[0] {
              serde_json::Value::String(phrase) => phrase.clone(),
              _ => return Err(anyhow::anyhow!("google translate returned unexpected format. response_body={:?}", response))
            });
          }

          let translated_text = phrases.join("");

          info!("translation={}", &translated_text);
          
          Ok(translated_text)
        },
        _ => Err(anyhow::anyhow!("google translate returned unexpected format. response_body={:?}", response))
      }
  }
}

