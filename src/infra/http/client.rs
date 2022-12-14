use anyhow::Result;
use async_trait::async_trait;

use crate::contracts::{
  self,
  http::{GetOptions, GetResponse, PostOptions, PostResponse},
};

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
impl contracts::http::HttpClient for ReqwestHttpClient {
  async fn post(
    &self,
    url: &str,
    body: Vec<u8>,
    options: Option<PostOptions>,
  ) -> Result<PostResponse> {
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

    let body = request_builder.body(body).send().await?.bytes().await?;

    Ok(PostResponse { body })
  }

  async fn get(&self, url: &str, options: Option<GetOptions>) -> Result<GetResponse> {
    let mut request_builder = self.client.get(url);

    if let Some(options) = options {
      if let Some(headers) = options.headers {
        for (key, value) in headers.into_iter() {
          request_builder = request_builder.header(key, value);
        }
      }

      if let Some(query) = options.query {
        request_builder = request_builder.query(&query);
      }

      if let Some(timeout) = options.timeout {
        request_builder = request_builder.timeout(timeout);
      }
    }

    let response = request_builder.send().await?;
    let headers = response.headers().clone();

    Ok(GetResponse {
      body: response.bytes().await?,
      headers,
    })
  }
}
