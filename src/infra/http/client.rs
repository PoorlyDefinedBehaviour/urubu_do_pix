use crate::contracts::{self, PostOptions, PostResponse};
use anyhow::Result;
use async_trait::async_trait;

pub struct ReqwestHttpClient {
  client: reqwest::Client,
}

impl ReqwestHttpClient {
  pub fn new() -> Self {
    Self {
      client: reqwest::Client::new(),
    }
  }
}

#[async_trait]
impl contracts::HttpClient for ReqwestHttpClient {
  async fn post(&self, url: &str, options: Option<PostOptions>) -> Result<PostResponse> {
    let mut request_builder = self.client.post(url);

    if let Some(options) = options {
      if let Some(headers) = options.headers {
        for (key, value) in headers.into_iter() {
          request_builder = request_builder.header(key, value);
        }
      }

      if let Some(timeout) = options.timeout {
        request_builder = request_builder.timeout(timeout);
      }
    }

    let body = request_builder.send().await?.bytes().await?;

    Ok(PostResponse { body })
  }
}
