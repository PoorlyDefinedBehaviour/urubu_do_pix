use anyhow::{Context as anyhowContext, Ok, Result};
use serenity::{client::Context, model::channel::Message};
use songbird::Songbird;
use std::fmt::Debug;
use std::{ffi::OsStr, path::Path, sync::Arc};
use tracing::info;

use crate::utils::check_message;

pub async fn get_songbird_manager(ctx: &Context) -> Result<Arc<Songbird>> {
  let manager = songbird::get(ctx)
    .await
    .context("Faild to get songbird instance")?;

  Ok(manager)
}

async fn join_channel(ctx: &Context, msg: &Message) -> Result<()> {
  let manager = get_songbird_manager(ctx).await?;

  let guild = msg.guild(ctx).context("Failed to get guild")?;
  let guild_id = guild.id;

  let user_voice_channel_id = guild
    .voice_states
    .get(&msg.author.id)
    .and_then(|vs| vs.channel_id);

  let user_voice_channel_id = match user_voice_channel_id {
    None => {
      check_message(msg.reply(ctx, "tu nao ta em call dog").await);
      return Ok(());
    }
    Some(channel_id) => channel_id,
  };

  let bot_voice_channel_id = guild
    .voice_states
    .get(&ctx.cache.current_user_id())
    .and_then(|vs| vs.channel_id);

  match bot_voice_channel_id {
    Some(bot_voice_channel_id) => {
      if bot_voice_channel_id != user_voice_channel_id {
        check_message(msg.reply(ctx, "ja to em outra call dog").await);
        return Ok(());
      }
    }
    None => (),
  };

  let _ = manager.join(guild_id, user_voice_channel_id).await;

  info!("joined channel. voice_channel_id={}", user_voice_channel_id);

  Ok(())
}

#[tracing::instrument(skip_all, fields(link = ?link))]
pub async fn play_audio<P: AsRef<OsStr> + Debug>(
  ctx: &Context,
  msg: &Message,
  link: P,
) -> Result<()> {
  join_channel(ctx, msg).await?;

  let manager = get_songbird_manager(ctx).await?;

  let guild = msg.guild(ctx).context("Failed to get guild")?;
  let guild_id = guild.id;

  let guild_lock = manager.get(guild_id).context("Unable to get guild lock")?;

  info!("Acquired guild lock");

  let mut handler = guild_lock.lock().await;

  let input = songbird::input::ffmpeg(link)
    .await
    .context("Error reading mp3 file")?;

  let track_handle = handler.play_source(input);

  track_handle.play().context("Error playing track")?;

  info!("Playing input");

  Ok(())
}

pub async fn play_local_audio(ctx: &Context, msg: &Message, file_name: &str) -> Result<()> {
  // O base path deve ser relativo ao current_dir, nao me pergunte pq
  let file_path = "./assets/".to_owned() + file_name;
  let file_path = Path::new(&file_path);

  if !file_path.exists() {
    check_message(msg.reply(ctx, "Esse arquivo nao existe").await);
    return Ok(());
  }

  play_audio(ctx, msg, file_path).await
}

pub async fn handler(ctx: &Context, msg: &Message, args_vec: Vec<&str>) -> Result<()> {
  let mut args = args_vec.into_iter();

  let sub_command = match args.next() {
    None => {
      check_message(msg.reply(ctx, "Tu nao passou um subcomando").await);
      return Ok(());
    }
    Some(v) => v,
  };

  match sub_command {
    "playlink" => {
      let link = args.next();

      match link {
        Some(link) => {
          play_audio(ctx, msg, link).await?;
          Ok(())
        }
        None => {
          check_message(msg.reply(ctx, "Faltou o link ae dog").await);
          Ok(())
        }
      }
    }
    "playlocal" => {
      let file_name = args.next();

      match file_name {
        Some(file_name) => {
          play_local_audio(ctx, msg, file_name).await?;
          Ok(())
        }
        None => {
          check_message(msg.reply(ctx, "Faltou o nome do arquivo ae dog").await);
          Ok(())
        }
      }
    }
    _ => {
      check_message(msg.reply(ctx, "Command not found").await);
      Ok(())
    }
  }
}
