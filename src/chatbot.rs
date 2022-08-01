use std::{collections::HashSet, fmt::Write, time::Duration};

use anyhow::Result;
use serenity::{
  client::Context,
  model::{channel::Message, id::ChannelId},
};

use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::{Mutex, RwLock};
use tracing::{error, info};

use crate::{
  audio, text_generation::TextGenerator, translation::Translation, tts::Tts, utils::env_key,
};

pub struct ChatBot {
  /// TODO: doc
  chat_bot_text: Mutex<String>,
  /// The set of text channels that the bot will interact with messages.
  text_channels: RwLock<HashSet<ChannelId>>,
  /// Push a message into this channel to play it in the voice chat.
  voice_chat_reply_sender: Sender<VoiceChatReply>,
  _voice_chat_reply_thread_handle: tokio::task::JoinHandle<()>,
  tts: Tts,
  text_generator: TextGenerator,
  translation: Translation,
}

/// The maximum number of voice channel voice messages that can be in the queue.
const MAX_VOICE_CHAT_REPLY_QUEUE_LENGTH: usize = 256;

struct VoiceChatReply {
  audio_file_url: String,
  ctx: Context,
  msg: Message,
}

impl std::fmt::Debug for VoiceChatReply {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("VoiceChatReply")
      .field("audio_file_url", &self.audio_file_url)
      .field("ctx", &"DOES NOT IMPLEMENT DEBUG")
      .field("msg", &self.msg)
      .finish()
  }
}

impl ChatBot {
  pub fn new(tts: Tts, text_generator: TextGenerator, translation: Translation) -> Self {
    let (sender, receiver) = tokio::sync::mpsc::channel(MAX_VOICE_CHAT_REPLY_QUEUE_LENGTH);

    // Spawn a thread to send voice chat audio.
    let handle = tokio::spawn(ChatBot::send_voice_chat_reply(receiver));

    Self {
      chat_bot_text: Mutex::new(env_key("CHAIML_INITIAL_CONTEXT").unwrap()),
      tts,
      text_generator,
      translation,
      text_channels: RwLock::new(HashSet::new()),
      _voice_chat_reply_thread_handle: handle,
      voice_chat_reply_sender: sender,
    }
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
    let track_handle =
      audio::play_audio(&message.ctx, &message.msg, message.audio_file_url).await?;

    let metadata = track_handle.metadata();

    tokio::time::sleep(metadata.duration.unwrap() + Duration::from_secs(1)).await;

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
      *context = context[context.len() - MAX_CONVERSARTION_HISTORY_LEN..].to_string();
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

    let bot_message_in_english = self.text_generator.generate(chat_bot_text.clone()).await?;

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

    let (reply_result, create_audio_result) = tokio::join!(
      msg.reply(ctx, &answer),
      self.tts.create_audio(bot_message_in_portuguese.clone())
    );

    if let Err(err) = reply_result {
      error!("error replying to message. error={:?}", err);
    }

    let audio_file_url = create_audio_result?;

    self
      .voice_chat_reply_sender
      .send(VoiceChatReply {
        ctx: ctx.clone(),
        msg: msg.clone(),
        audio_file_url,
      })
      .await?;

    Ok(())
  }
}
