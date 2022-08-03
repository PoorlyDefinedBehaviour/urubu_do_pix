use anyhow::Result;
use async_trait::async_trait;
use bytes::Bytes;
use reqwest::header::HeaderMap;
use std::time::Duration;

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait TextToSpeech: Send + Sync {
  async fn create_audio(&self, text: String) -> Result<Vec<String>>;
}

#[derive(Debug)]
pub struct PostOptions {
  pub headers: Option<Vec<(String, String)>>,
  pub timeout: Option<Duration>,
}

#[derive(Debug)]
pub struct PostResponse {
  pub body: Bytes,
}

#[derive(Debug)]
pub struct GetOptions {
  pub headers: Option<Vec<(String, String)>>,
  pub query: Option<Vec<(String, String)>>,
  pub timeout: Option<Duration>,
}

#[derive(Debug)]
pub struct GetResponse {
  pub headers: HeaderMap,
  pub body: Bytes,
}

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait HttpClient: Send + Sync {
  async fn post(
    &self,
    url: &str,
    body: Vec<u8>,
    options: Option<PostOptions>,
  ) -> Result<PostResponse>;
  async fn get(&self, url: &str, options: Option<GetOptions>) -> Result<GetResponse>;
}
