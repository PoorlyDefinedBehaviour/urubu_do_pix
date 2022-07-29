use anyhow::{Context as anyhowContext, Result};
use rand::Rng;
use serenity::async_trait;
use serenity::client::Context;
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::prelude::*;
use songbird::SerenityInit;
use tracing::{error, info};
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_subscriber::{filter::EnvFilter, layer::SubscriberExt, Registry};
use tracing_tree::HierarchicalLayer;

mod command;

struct Bot;

impl Bot {
  #[tracing::instrument(skip_all, fields(
    author_id = %msg.author.id,
    author_name = %msg.author.name,
    message_content = %msg.content
  ))]
  async fn command_handler(ctx: Context, msg: &Message) {
    if msg.is_own(&ctx) {
      return;
    }

    if let Err(err) = sex(&ctx, msg).await {
      error!("error executing sexo handler. error={:?}", err);
    }

    let prefix = "b!";
    let is_command = msg.content.starts_with(prefix);

    if !is_command {
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
      cmd => {
        info!("unknown command. command={}", cmd);
        Ok(())
      }
    };

    if let Err(err) = result {
      error!("error executing command. command={} error={:?}", cmd, err);
    }
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
  let manager = songbird::get(ctx).await.context("songbird faio dog")?;

  let guild = msg.guild(&ctx).context("!!ZANDERS Failed to get guild")?;
  let guild_id = guild.id;

  let voice_channel_id = guild
    .voice_states
    .get(&msg.author.id)
    .and_then(|vs| vs.channel_id);

  let voice_channel_id = match voice_channel_id {
    None => {
      let _ = msg.reply(&ctx, "tu nao ta em call dog").await;
      return Ok(());
    }
    Some(channel_id) => channel_id,
  };

  let _ = manager.join(guild_id, voice_channel_id).await;

  info!("joined channel. voice_channel_id={}", voice_channel_id);

  let guild_lock = manager
    .get(guild_id)
    .context("!!ZANDERS unable to get guild lock")?;

  info!("acquired guild lock");

  let mut handler = guild_lock.lock().await;

  let input = songbird::ytdl("https://www.youtube.com/watch?v=Yb82Uck6I6Q").await?;

  // let input = songbird::input::ffmpeg("../assets/yo_zanders.mp3")
  //   .await
  //   .context("error reading mp3 file")?;

  info!("input loaded {:?}", input);

  let track_handle = handler.play_source(input);

  // vou coringar se for isso
  track_handle.play().context("error playing track")?;

  info!("playing input");

  Ok(())
}

#[async_trait]
impl EventHandler for Bot {
  async fn ready(&self, _: Context, ready: Ready) {
    info!("Bot is ready as {}", ready.user.name)
  }

  async fn message(&self, ctx: Context, msg: Message) {
    Bot::command_handler(ctx, &msg).await;
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
  .event_handler(Bot)
  .register_songbird()
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
