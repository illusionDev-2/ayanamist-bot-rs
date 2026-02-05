use crate::verify::common::{
    ANSWER_PREFIX, COLOR_AQUA, COLOR_FAIL, COLOR_WHITE, FOOTER_ICON_URL, START_ID,
};
use crate::{Data, Error};
use dashmap::DashMap;
use once_cell::sync::Lazy;
use poise::serenity_prelude as serenity;
use rand::Rng;
use rand::seq::SliceRandom;
use std::time::{Duration, Instant};

const TIME_LIMIT: Duration = Duration::from_secs(20);

pub async fn handle_component(
    ctx: &serenity::Context,
    data: &Data,
    interaction: &serenity::ComponentInteraction,
) -> Result<(), Error> {
    let id = interaction.data.custom_id.as_str();

    if id == START_ID {
        return on_start(ctx, interaction).await;
    }
    if let Some(rest) = id.strip_prefix(ANSWER_PREFIX) {
        return on_answer(ctx, data, interaction, rest).await;
    }
    Ok(())
}

async fn on_start(
    ctx: &serenity::Context,
    interaction: &serenity::ComponentInteraction,
) -> Result<(), Error> {
    let user_id = interaction.user.id;

    if let Some(existing) = CHALLENGES.get(&user_id) {
        if Instant::now() <= existing.expires_at {
            interaction
                .create_response(
                    ctx,
                    serenity::CreateInteractionResponse::Message(
                        serenity::CreateInteractionResponseMessage::new()
                            .content("すでに挑戦中です。")
                            .ephemeral(true),
                    ),
                )
                .await?;
            return Ok(());
        }
        CHALLENGES.remove(&user_id);
    }

    let (a, b, correct, mut choices) = {
        let mut rng = rand::thread_rng();
        let a = rng.gen_range(2..=9);
        let b = rng.gen_range(2..=9);
        let correct = a * b;

        let mut choices = vec![correct];
        while choices.len() < 5 {
            let d = rng.gen_range(2..=81);
            if !choices.contains(&d) {
                choices.push(d);
            }
        }
        choices.shuffle(&mut rng);
        (a, b, correct, choices)
    };

    CHALLENGES.insert(
        user_id,
        Challenge {
            correct,
            expires_at: Instant::now() + TIME_LIMIT,
        },
    );

    let embed = serenity::CreateEmbed::new()
        .color(COLOR_WHITE)
        .title("認証チャレンジ")
        .description(format!("**{a} × {b} = ?**"))
        .footer(serenity::CreateEmbedFooter::new("制限時間：20秒"));

    let buttons = choices
        .drain(..)
        .map(|n| {
            serenity::CreateButton::new(format!("{ANSWER_PREFIX}{n}"))
                .label(n.to_string())
                .style(serenity::ButtonStyle::Secondary)
        })
        .collect();

    interaction
        .create_response(
            ctx,
            serenity::CreateInteractionResponse::Message(
                serenity::CreateInteractionResponseMessage::new()
                    .embed(embed)
                    .components(vec![serenity::CreateActionRow::Buttons(buttons)])
                    .ephemeral(true),
            ),
        )
        .await?;

    Ok(())
}

#[derive(Clone)]
struct Challenge {
    correct: u32,
    expires_at: Instant,
}

static CHALLENGES: Lazy<DashMap<serenity::UserId, Challenge>> = Lazy::new(DashMap::new);

async fn on_answer(
    ctx: &serenity::Context,
    data: &Data,
    interaction: &serenity::ComponentInteraction,
    answered_str: &str,
) -> Result<(), Error> {
    let user_id = interaction.user.id;
    let answered: u32 = match answered_str.parse() {
        Ok(v) => v,
        Err(_) => return Ok(()),
    };

    let Some(ch) = CHALLENGES.get(&user_id).map(|v| v.clone()) else {
        return Ok(());
    };

    if Instant::now() > ch.expires_at {
        CHALLENGES.remove(&user_id);

        let embed = serenity::CreateEmbed::new()
            .color(COLOR_FAIL)
            .title("⌛ 時間切れ")
            .description("もう一度やり直してください。")
            .footer(serenity::CreateEmbedFooter::new("Ayanamist System").icon_url(FOOTER_ICON_URL));

        interaction
            .create_response(
                ctx,
                serenity::CreateInteractionResponse::Message(
                    serenity::CreateInteractionResponseMessage::new()
                        .embed(embed)
                        .ephemeral(true),
                ),
            )
            .await?;
        return Ok(());
    }

    if answered != ch.correct {
        CHALLENGES.remove(&user_id);

        let embed = serenity::CreateEmbed::new()
            .color(COLOR_FAIL)
            .title("❌ 不正解")
            .description("もう一度やり直してください。")
            .footer(serenity::CreateEmbedFooter::new("Ayanamist System").icon_url(FOOTER_ICON_URL));

        interaction
            .create_response(
                ctx,
                serenity::CreateInteractionResponse::Message(
                    serenity::CreateInteractionResponseMessage::new()
                        .embed(embed)
                        .ephemeral(true),
                ),
            )
            .await?;
        return Ok(());
    }

    let Some(guild_id) = interaction.guild_id else {
        return Ok(());
    };

    let member = guild_id.member(ctx, user_id).await?;
    member
        .add_role(ctx, data.config.verify.verify_role_id)
        .await?;
    CHALLENGES.remove(&user_id);

    let embed = serenity::CreateEmbed::new()
        .color(COLOR_AQUA)
        .title("✅ 認証成功")
        .description("ロールを付与しました。")
        .footer(serenity::CreateEmbedFooter::new("Ayanamist System").icon_url(FOOTER_ICON_URL));

    interaction
        .create_response(
            ctx,
            serenity::CreateInteractionResponse::Message(
                serenity::CreateInteractionResponseMessage::new()
                    .embed(embed)
                    .ephemeral(true),
            ),
        )
        .await?;

    Ok(())
}
