use std::fmt::Write;

use crate::contracts;
use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::{error, info};

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

fn split_str_and_include_separator(text: &str) -> Vec<(Option<char>, String)> {
  let mut pieces = vec![];

  let mut buffer = String::new();

  for (i, character) in text.chars().enumerate() {
    if character == '.' {
      pieces.push((Some('.'), std::mem::take(&mut buffer)));
    } else if character == ',' {
      pieces.push((Some(','), std::mem::take(&mut buffer)));
    } else {
      buffer.push(character);

      if i == text.len() - 1 && !buffer.is_empty() {
        pieces.push((None, std::mem::take(&mut buffer)));
      }
    }
  }

  pieces
}

/// The tts api accepts only 200 characters at a time, so if we get a text thats longer than that
/// we split the text using the punctuation.
fn divide_text_into_chunks(text: &str) -> Result<Vec<String>> {
  let mut chunks = vec![];

  let mut buffer = String::new();

  let pieces = split_str_and_include_separator(text);

  for (i, (separator, piece)) in pieces.iter().enumerate() {
    if buffer.len() + piece.len() > 200 {
      chunks.push(std::mem::take(&mut buffer));
    }

    match separator {
      None => buffer.push_str(piece),
      Some(separator) => {
        write!(&mut buffer, "{}{}", piece, separator)?;
      }
    }

    if i == pieces.len() - 1 && !buffer.is_empty() {
      chunks.push(std::mem::take(&mut buffer));
    }
  }

  Ok(chunks)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_split_str_and_include_separator() {
    let input = "Once upon a time, in a far away swamp, there lived an ogre named Shrek (Mike Myers) whose precious solitude is suddenly shattered by an invasion of annoying fairy tale characters.";
    let expected = vec![
      (
          Some(
              ',',
          ),
          String::from("Once upon a time"),
      ),
      (
          Some(
              ',',
          ),
          String::from(" in a far away swamp"),
      ),
      (
          Some(
              '.',
          ),
          String::from(" there lived an ogre named Shrek (Mike Myers) whose precious solitude is suddenly shattered by an invasion of annoying fairy tale characters"),
      ),
    ];
    assert_eq!(expected, split_str_and_include_separator(input));
  }

  #[test]
  fn test_divide_text_into_chunks() {
    let tests = vec![(
      r#"
      Once upon a time, in a far away swamp, there lived an ogre named Shrek (Mike Myers) whose precious solitude is suddenly shattered by an invasion of annoying fairy tale characters.
      They were all banished from their kingdom by the evil Lord Farquaad (John Lithgow).
      Determined to save their home -- not to mention his -- Shrek cuts a deal with Farquaad and sets out to rescue Princess Fiona (Cameron Diaz) to be Farquaad's bride.
      Rescuing the Princess may be small compared to her deep, dark secret.
    "#,
    vec![
      "\n      Once upon a time, in a far away swamp, there lived an ogre named Shrek (Mike Myers) whose precious solitude is suddenly shattered by an invasion of annoying fairy tale characters.",
      "\n      They were all banished from their kingdom by the evil Lord Farquaad (John Lithgow).",
      "\n      Determined to save their home -- not to mention his -- Shrek cuts a deal with Farquaad and sets out to rescue Princess Fiona (Cameron Diaz) to be Farquaad's bride.",
      "\n      Rescuing the Princess may be small compared to her deep, dark secret.\n    ",
    ]
    ),
    (
      "",
      vec![]
    ),
    (
      "Once upon. a time in. a far away swamp. there lived an ogre. named Shrek. ",
      vec!["Once upon. a time in. a far away swamp. there lived an ogre. named Shrek. "]
    )
    ];

    for (input, expected) in tests {
      assert_eq!(expected, divide_text_into_chunks(input).unwrap());
    }
  }
}
