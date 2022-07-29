use anyhow::Result;
use serenity::async_trait;
use serenity::client::Context;
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::prelude::*;
use tracing::info;
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_subscriber::{filter::EnvFilter, layer::SubscriberExt, Registry};

struct Handler;

#[async_trait]
impl EventHandler for Handler {
  async fn ready(&self, _: Context, ready: Ready) {
    info!("Bot is ready as {}", ready.user.name)
  }

  async fn message(&self, _ctx: Context, msg: Message) {
    info!("MSG {:?}", msg)
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
    .with(JsonStorageLayer)
    .with(bunyan_formatting_layer);

  tracing::subscriber::set_global_default(subscriber).unwrap();

  let token = env_key("DISCORD_TOKEN")?;

  let intents = GatewayIntents::non_privileged() | GatewayIntents::MESSAGE_CONTENT;

  let mut client = Client::builder(token, intents)
    .event_handler(Handler)
    .await
    .expect("Failed to create bot");

  info!("starting bot");

  if let Err(why) = client.start().await {
    info!("An error occurred while running the client: {:?}", why);
  }

  Ok(())
}

fn env_key(key: &str) -> Result<String, String> {
  let value = std::env::var(key).ok();
  match value {
    None => Err(format!("missing env variable: {}", key)),
    Some(value) => Ok(value),
  }
}
