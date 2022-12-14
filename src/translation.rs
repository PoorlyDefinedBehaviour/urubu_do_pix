use std::{time::Duration, sync::Arc};

use retry::{Retry, ExponentialBackoff};
use tracing::info;
use anyhow::Result;

use crate::contracts::{self, http::GetOptions};

pub struct Translation {
  http_client: Arc<dyn contracts::http::HttpClient>
}

impl Translation {
  pub fn new(http_client: Arc<dyn contracts::http::HttpClient>) -> Self {
    Self {
      http_client
    }
  }

  #[tracing::instrument(skip_all, fields(
    from_lang = %from_lang, 
    to_lang = %to_lang,
    text = %text
  ))]
  pub async fn translate(&self, text: &str, from_lang: &str, to_lang: &str) -> Result<String> {
    Retry::new()
      .retries(3)
      .backoff(ExponentialBackoff::recommended())
      .exec(|| async {
        let response= self.http_client.get("https://translate.googleapis.com/translate_a/single?client=gtx", Some(GetOptions{
          headers: None, 
          query: Some(vec![
            ("sl".to_string(), from_lang.to_string()),  
            ("tl".to_string() ,to_lang.to_string()),
            ("dt".to_string(),"t".to_string()),
            ("q".to_string(), text.to_string())
          ]),
          timeout: Some(Duration::from_secs(30)),
        }))
        .await?;

        let body: serde_json::Value = serde_json::from_slice(&response.body)?;
   
        match &body[0] {
          serde_json::Value::Array(translations) => {
            let mut phrases = Vec::with_capacity(translations.len());

            for translation in translations.iter() {
              phrases.push(match &translation[0] {
                serde_json::Value::String(phrase) => phrase.clone(),
                _ => return Err(
                  anyhow::anyhow!(
                    "google translate returned unexpected format. response_body={:?} headers={:?}", 
                    body, &response.headers
                  ))
                });
              }

              let translated_text = phrases.join("");

              info!("translation={}", &translated_text);
         
              Ok(translated_text)
            },
            _ => Err(anyhow::anyhow!("google translate returned unexpected format. response_body={:?}", response))
          }
        })
    .await
  }
}

