use anyhow::Result;
use rand::Rng;
use serenity::async_trait;
use serenity::client::Context;
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::prelude::*;
use serenity::model::id::ChannelId;
use songbird::SerenityInit;
use tokio::sync::{RwLock, Mutex};
use tracing::{error, info};
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_subscriber::{filter::EnvFilter, layer::SubscriberExt, Registry};
use tracing_tree::HierarchicalLayer;

mod audio;
mod music;
mod text_generation;
mod translation;
mod tts;
mod utils;

use text_generation::TextGenerator;
use translation::Translation;
use tts::Tts;

use crate::utils::env_key;

struct Bot {
  chat_bot_text: Mutex<String>,
  tts: Tts,
  text_generator: TextGenerator,
  translation: Translation,
  /// The text channel that the chat bot will have a conversation in.
  chatbot_channel: RwLock<Option<ChannelId>>,
}

impl Bot {
  pub fn new(tts: Tts, text_generator: TextGenerator, translation: Translation) -> Self {
    Self {
      chat_bot_text: Mutex::new(env_key("CHAIML_INITIAL_CONTEXT").unwrap()),
      tts,
      text_generator,
      translation,
      chatbot_channel: RwLock::new(None),
    }
  }

  #[tracing::instrument(skip_all, fields(
    author_id = %msg.author.id,
    author_name = %msg.author.name,
    message_content = %msg.content
  ))]
  async fn command_handler(&self, ctx: Context, msg: &Message) {
    if msg.is_own(&ctx) {
      return;
    }

    if msg.author.bot {
      return;
    }

    let prefix = "b!";
    let is_command = msg.content.starts_with(prefix);

    if !is_command {
      if let Err(err) = sex(&ctx, msg).await {
        error!("error executing sexo handler. error={:?}", err);
      }

      if let Err(err) = self.conversation_bot(&ctx, msg).await {
        error!("error executing conversation_bot handler. error={:?}", err);
      }

      return;
    }

    let mut args = msg.content[prefix.len()..].split_whitespace();

    let cmd = match args.next() {
      None => {
        info!("Ta maluco porra");
        return;
      }
      Some(v) => v,
    };

    let result = match cmd {
      "echo" => echo(&ctx, msg, args.collect::<Vec<_>>().join(" ")).await,
      "zanders" => zanders(&ctx, msg).await,
      "sound" => audio::handler(&ctx, msg, args.collect::<Vec<_>>()).await,
      "chatbot" => self.set_chatbot_current_text_channel(&ctx, msg).await,
      cmd => {
        info!("unknown command. command={}", cmd);
        Ok(())
      }
    };

    if let Err(err) = result {
      error!("error executing command. command={} error={:?}", cmd, err);
    }
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

  #[tracing::instrument(name = "conversation_bot", skip_all)]
  pub async fn conversation_bot(&self, ctx: &Context, msg: &Message) -> Result<()> {
    // User must use the `chatbot` command to enable the bot in the channel.
    let chatbot_channel = self.chatbot_channel.read().await;
    if *chatbot_channel != Some(msg.channel_id) {
      return Ok(());
    }

    let message_in_english: String = self.translation.translate(&msg.content, "pt", "en").await?;

    let mut chat_bot_text = self.chat_bot_text.lock().await;

    // Save the chat bot response so we can use it as context later.
    chat_bot_text.push_str(&format!("Me: {}\n", &message_in_english));

    self.truncate_chat_bot_text_length(&mut chat_bot_text);

    let bot_message_in_english = self.text_generator.generate(chat_bot_text.clone()).await?;

    // Add bot response to context.
    chat_bot_text.push_str(&format!("Eliza: {}\n", &bot_message_in_english));

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

    let file_url = create_audio_result?;

    audio::play_audio(ctx, msg, file_url).await?;

    Ok(())
  }

  #[tracing::instrument(name = "set_chatbot_current_text_channel", skip_all)]
  async fn set_chatbot_current_text_channel(&self, ctx: &Context, msg: &Message) -> Result<()> {
    *self.chatbot_channel.write().await = Some(msg.channel_id);

    msg.reply(ctx, "chatbot joined the channel").await?;

    Ok(())
  }
}

#[tracing::instrument(name = "sexo", skip_all)]
async fn sex(ctx: &Context, msg: &Message) -> Result<()> {
  if msg.content.contains("sexo") && rand::thread_rng().gen_range(0..=1000) <= 10 {
    let _ = msg
      .reply(&ctx, "só um sexozinho agora pprt :hot_face:")
      .await;
  }

  Ok(())
}

#[tracing::instrument(name = "echo", skip_all)]
async fn echo(ctx: &Context, msg: &Message, arg: String) -> Result<()> {
  msg.reply(ctx, arg).await?;

  Ok(())
}

#[tracing::instrument(name = "zanders", skip_all)]
async fn zanders(ctx: &Context, msg: &Message) -> Result<()> {
  audio::play_local_audio(ctx, msg, "yo_zanders.mp3").await?;

  Ok(())
}

#[async_trait]
impl EventHandler for Bot {
  async fn ready(&self, _: Context, ready: Ready) {
    info!("Bot is ready as {}", ready.user.name)
  }

  async fn message(&self, ctx: Context, msg: Message) {
    self.command_handler(ctx, &msg).await;
  }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  dotenv::dotenv().expect("error reading .env file");

  let (non_blocking_writer, _guard) = tracing_appender::non_blocking(std::io::stdout());

  let app_name = concat!(env!("CARGO_PKG_NAME"), "-", env!("CARGO_PKG_VERSION")).to_string();

  let bunyan_formatting_layer = BunyanFormattingLayer::new(app_name, non_blocking_writer);

  let subscriber = Registry::default()
    .with(EnvFilter::from_env("RUST_LOG"))
    // .with(JsonStorageLayer);
    .with(HierarchicalLayer::new(2))
    .with(bunyan_formatting_layer);

  tracing::subscriber::set_global_default(subscriber).unwrap();

  let token = env_key("DISCORD_TOKEN")?;

  let mut client = Client::builder(
    token,
    GatewayIntents::non_privileged()
      | GatewayIntents::MESSAGE_CONTENT
      | GatewayIntents::GUILD_VOICE_STATES,
  )
  .event_handler(Bot::new(
    Tts::new(),
    TextGenerator::new(),
    Translation::new(),
  ))
  .register_songbird()
  .await
  .expect("Failed to create bot");

  info!("starting bot");

  if let Err(why) = client.start().await {
    info!("An error occurred while running the client: {:?}", why);
  }

  Ok(())
}

