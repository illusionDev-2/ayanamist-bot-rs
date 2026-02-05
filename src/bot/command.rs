use crate::{Context, Error};

/// pong
#[poise::command(slash_command, guild_only)]
pub async fn ping(ctx: Context<'_>) -> Result<(), Error> {
    ctx.say("pong 🦀").await?;
    Ok(())
}
