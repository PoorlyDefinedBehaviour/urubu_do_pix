use std::{
  collections::HashSet,
  fmt::Write,
  sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
  },
  time::Duration,
};

use anyhow::Result;
use serenity::{
  client::Context,
  model::{channel::Message, id::ChannelId},
};

use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::{Mutex, RwLock};
use tracing::{error, info};

use crate::{
  audio, contracts, text_generation::TextGenerator, translation::Translation, utils::env_key,
};

pub struct ChatBot {
  /// TODO: doc
  chat_bot_text: Mutex<String>,
  /// The set of text channels that the bot will interact with messages.
  text_channels: RwLock<HashSet<ChannelId>>,
  /// Will the bot reply to messages by playing audio?
  voice_chat_enabled: AtomicBool,
  /// Push a message into this channel to play it in the voice chat.
  voice_chat_reply_sender: Sender<VoiceChatReply>,
  _voice_chat_reply_thread_handle: tokio::task::JoinHandle<()>,
  tts: Arc<dyn contracts::TextToSpeech>,
  text_generator: TextGenerator,
  translation: Translation,
}

/// The maximum number of voice channel voice messages that can be in the queue.
const MAX_VOICE_CHAT_REPLY_QUEUE_LENGTH: usize = 256;

struct VoiceChatReply {
  audio_file_urls: Vec<String>,
  ctx: Context,
  msg: Message,
}

impl std::fmt::Debug for VoiceChatReply {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("VoiceChatReply")
      .field("audio_file_urls", &self.audio_file_urls)
      .field("ctx", &"DOES NOT IMPLEMENT DEBUG")
      .field("msg", &self.msg)
      .finish()
  }
}

impl ChatBot {
  pub fn new(
    tts: Arc<dyn contracts::TextToSpeech>,
    text_generator: TextGenerator,
    translation: Translation,
  ) -> Self {
    let (sender, receiver) = tokio::sync::mpsc::channel(MAX_VOICE_CHAT_REPLY_QUEUE_LENGTH);

    // Spawn a thread to send voice chat messages.
    let handle = tokio::spawn(ChatBot::send_voice_chat_reply(receiver));

    Self {
      chat_bot_text: Mutex::new(env_key("CHAIML_INITIAL_CONTEXT").unwrap()),
      tts,
      text_generator,
      translation,
      text_channels: RwLock::new(HashSet::new()),
      _voice_chat_reply_thread_handle: handle,
      voice_chat_reply_sender: sender,
      voice_chat_enabled: AtomicBool::new(true),
    }
  }

  #[tracing::instrument(skip_all)]
  pub fn enable_voice(&self) {
    self.voice_chat_enabled.store(true, Ordering::Relaxed);
  }

  #[tracing::instrument(skip_all)]
  pub fn disable_voice(&self) {
    self.voice_chat_enabled.store(false, Ordering::Relaxed);
  }

  #[tracing::instrument(skip_all)]
  pub fn is_voice_enabled(&self) -> bool {
    self.voice_chat_enabled.load(Ordering::Relaxed)
  }

  /// Adds the bot the text channel where the message has been sent to.
  #[tracing::instrument(skip_all)]
  pub async fn join_text_channel(&self, ctx: &Context, msg: &Message) -> Result<()> {
    self.text_channels.write().await.insert(msg.channel_id);

    msg.reply(ctx, "chatbot joined the channel").await?;

    Ok(())
  }

  #[tracing::instrument(skip_all)]
  async fn send_voice_chat_reply(mut receiver: Receiver<VoiceChatReply>) {
    loop {
      match receiver.recv().await {
        None => {
          info!("conversation bot voice chat reply channel has been closed");
          // Channel has been closed.
          break;
        }
        Some(message) => {
          info!("playing voice chat reply");

          if let Err(err) = Self::do_send_voice_chat_reply(message).await {
            error!("error sending voice chat reply. error={:?}", err);
          }
        }
      }
    }
  }

  #[tracing::instrument(skip_all)]
  async fn do_send_voice_chat_reply(message: VoiceChatReply) -> Result<()> {
    for audio_file_chunk_url in message.audio_file_urls.into_iter() {
      let track_handle =
        audio::play_audio(&message.ctx, &message.msg, audio_file_chunk_url).await?;

      let metadata = track_handle.metadata();

      tokio::time::sleep(metadata.duration.unwrap() + Duration::from_millis(500)).await;
    }

    Ok(())
  }

  /// Ensure the text sent to the chat bot is not too long because the api may
  /// get slow if it is.
  #[tracing::instrument(skip_all)]
  fn truncate_chat_bot_text_length(&self, context: &mut String) {
    // The api has a limit of 5000 characters but the chat bot gets slow at around 3500.
    const MAX_CONVERSARTION_HISTORY_LEN: usize = 2750;
    if context.len() > MAX_CONVERSARTION_HISTORY_LEN {
      info!(
        "pruning chat bot context. len_before_pruning={}",
        context.len()
      );

      *context = context
        .chars()
        .take(context.len() - MAX_CONVERSARTION_HISTORY_LEN)
        .collect();
    }
  }

  /// Called whenever a message is sent.
  #[tracing::instrument(name = "conversation_bot", skip_all)]
  pub async fn on_message(&self, ctx: &Context, msg: &Message) -> Result<()> {
    if msg.is_own(ctx) {
      return Ok(());
    }

    // User must use the `chatbot` command to enable the bot in the channel.
    let text_channels = self.text_channels.read().await;
    if !text_channels.contains(&msg.channel_id) {
      return Ok(());
    }

    let message_in_english: String = self.translation.translate(&msg.content, "pt", "en").await?;

    let mut chat_bot_text = self.chat_bot_text.lock().await;

    // Save the chat bot response so we can use it as context later.
    writeln!(&mut chat_bot_text, "Me: {}", &message_in_english)?;

    self.truncate_chat_bot_text_length(&mut chat_bot_text);

    let bot_message_in_english = self.text_generator.generate(&chat_bot_text).await?;

    // Add bot response to context.
    writeln!(&mut chat_bot_text, "Eliza: {}", &bot_message_in_english)?;

    let bot_message_in_portuguese = self
      .translation
      .translate(&bot_message_in_english, "en", "pt")
      .await?;

    let answer = format!(
      "EN:{} \n\nPT: {}",
      bot_message_in_english, bot_message_in_portuguese
    );

    if let Err(err) = msg.reply(ctx, &answer).await {
      error!("error replying to message. error={:?}", err);
    }

    if !self.is_voice_enabled() {
      info!("voice chat is disabled");
      return Ok(());
    }

    let audio_file_urls = self
      .tts
      .create_audio(remove_links_from_text(&bot_message_in_portuguese))
      .await?;

    self
      .voice_chat_reply_sender
      .send(VoiceChatReply {
        ctx: ctx.clone(),
        msg: msg.clone(),
        audio_file_urls,
      })
      .await?;

    Ok(())
  }
}

fn remove_links_from_text(text: &str) -> String {
  text
    .split_whitespace()
    .filter(|s| !s.starts_with("http://") && !s.starts_with("https://"))
    .collect::<Vec<_>>()
    .join(" ")
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_remove_links_from_text() {
    let tests = vec![
      (
        "Eu: @d!music search https://www.youtube.com/watch?v=JW1p9j8HVXA",
        "Eu: @d!music search",
      ),
      (
        "D!Pesquisa de música https://www.youtube.com/watch?v=Ao8F3FypsbI",
        "D!Pesquisa de música",
      ),
      (
        "Eu: @d!music search https://www.youtube.com/watch?v=JW1p9j8HVXA",
        "Eu: @d!music search",
      ),
      (
        "D!Pesquisa de música https://www.youtube.com/watch?v=Ao8F3FypsbI",
        "D!Pesquisa de música",
      ),
      ("http://google.com", ""),
      ("https://google.com", ""),
      ("", ""),
      ("123", "123"),
      ("abc", "abc"),
    ];

    for (input, expected) in tests {
      assert_eq!(expected, remove_links_from_text(input), "input={}", input);
    }
  }
}
