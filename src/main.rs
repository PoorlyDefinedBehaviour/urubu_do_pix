use std::{str::SplitWhitespace, sync::Arc};

use anyhow::Result;
use chatbot::ChatBot;
use rand::Rng;
use serenity::async_trait;
use serenity::client::Context;
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::prelude::*;
use songbird::SerenityInit;
use tracing::{error, info};
use tracing_bunyan_formatter::BunyanFormattingLayer;
use tracing_subscriber::{filter::EnvFilter, layer::SubscriberExt, Registry};
use tracing_tree::HierarchicalLayer;

mod audio;
mod chatbot;
mod contracts;
mod infra;
mod text_generation;
mod translation;
mod tts;
mod utils;

use text_generation::TextGenerator;
use translation::Translation;
use tts::Tts;

use crate::{
  infra::{
    cache::{self, redis::RedisCache},
    http::client::ReqwestHttpClient,
  },
  text_generation::Config,
  utils::env_key,
};

struct Bot {
  chatbot: ChatBot,
}

impl Bot {
  pub fn new(chatbot: ChatBot) -> Self {
    Self { chatbot }
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

      if let Err(err) = self.chatbot.on_message(&ctx, msg).await {
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
      "chatbot" => self.chatbot(&ctx, msg, args).await,
      cmd => {
        info!("unknown command. command={}", cmd);
        Ok(())
      }
    };

    if let Err(err) = result {
      error!("error executing command. command={} error={:?}", cmd, err);
    }
  }

  #[tracing::instrument(name = "chatbot", skip_all)]
  async fn chatbot(
    &self,
    ctx: &Context,
    msg: &Message,
    mut args: SplitWhitespace<'_>,
  ) -> Result<()> {
    match args.next() {
      None => self.chatbot.join_text_channel(ctx, msg).await?,
      Some(subcommand) => match subcommand {
        "eliza" => {
          self
            .chatbot
            .set_user_history(msg.author.id.0, &env_key("CHAIML_INITIAL_CONTEXT")?)
            .await?;
          msg.reply(&ctx, "history set").await?;
        }
        "sethistory" => {
          self
            .chatbot
            .set_user_history(msg.author.id.0, &msg.content)
            .await?;
          msg.reply(&ctx, "history set").await?;
        }
        "history" => {
          msg
            .reply(
              &ctx,
              self
                .chatbot
                .conversation_history_for_user(msg.author.id.0)
                .await?,
            )
            .await?;
        }
        "voice" => {
          let arg = args.next();
          match arg {
            Some("enable") => {
              self.chatbot.enable_voice();
              msg.reply(&ctx, "voice chat enabled").await?;
            }
            Some("disable") => {
              self.chatbot.disable_voice();
              msg.reply(&ctx, "voice chat disabled").await?;
            }
            _ => {
              msg
                .reply(&ctx, format!("unexpected argument: {:?}", arg))
                .await?;
            }
          }
        }
        _ => {
          info!("unknown subcommand. subcommand={}", subcommand);
        }
      },
    }

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
    .with(HierarchicalLayer::new(1))
    .with(bunyan_formatting_layer);

  tracing::subscriber::set_global_default(subscriber).unwrap();

  let token = env_key("DISCORD_TOKEN")?;

  let mut client = Client::builder(
    token,
    GatewayIntents::non_privileged()
      | GatewayIntents::MESSAGE_CONTENT
      | GatewayIntents::GUILD_VOICE_STATES,
  )
  .event_handler(Bot::new(ChatBot::new(
    Arc::new(Tts::new()),
    TextGenerator::new(
      Config {
        chaiml_developer_uuid: env_key("CHAIML_DEVELOPER_UUID")?,
        chaiml_key: env_key("CHAIML_KEY")?,
      },
      Arc::new(ReqwestHttpClient::new()),
    ),
    Translation::new(Arc::new(ReqwestHttpClient::new())),
    Arc::new(RedisCache::new(cache::redis::Config {
      host: env_key("REDIS_HOST")?,
      port: env_key("REDIS_PORT")?.parse::<u16>()?,
      password: env_key("REDIS_PASSWORD")?,
    })?),
  )))
  .register_songbird()
  .await
  .expect("Failed to create bot");

  info!("starting bot");

  if let Err(why) = client.start().await {
    info!("An error occurred while running the client: {:?}", why);
  }

  Ok(())
}
