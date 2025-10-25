use crate::{
    fumo_context::FumoContext,
    utils::{
        interaction::InteractionCommand,
        searching::{find_beatmap_link, parse_beatmap_link},
    },
};
use eyre::Result;
use fumo_database::osu::{OsuDbMatchGame, OsuDbMatchScore};
use fumo_twilight::message::MessageBuilder;
use num_format::{Locale, ToFormattedString};
use osu_api::models::{
    OsuBeatmap, OsuGameMode, OsuMods, OsuUserExtended, UserId,
};
use std::fmt::Write;
use twilight_interactions::command::{CommandModel, CreateCommand};
use twilight_model::channel::message::Embed;
use twilight_util::builder::embed::{
    EmbedAuthorBuilder, EmbedBuilder, ImageSource,
};

use super::ListKind;

/// List all multiplayer scores
#[derive(CommandModel, CreateCommand, Debug)]
#[command(name = "compare")]
pub struct MultiplayerCompare {
    /// Lookup in all matches or only in tournament related
    pub kind: ListKind,

    /// Beatmap ID or beatmap link
    pub beatmap: Option<String>,

    /// User ID or username
    pub user: Option<String>,
}

impl MultiplayerCompare {
    pub async fn run(
        &self,
        ctx: &FumoContext,
        cmd: InteractionCommand,
    ) -> Result<()> {
        cmd.defer(ctx).await?;

        // 1. Try to extract user
        let user = match &self.user {
            Some(v) => UserId::from(v.as_str()),
            None => {
                // If not provided try to fetch from linked users
                let Some(user) = osu_user!(ctx, cmd) else {
                    let builder = MessageBuilder::new()
                        .content("No provided or linked user is found");

                    cmd.update(ctx, &builder).await?;

                    return Ok(());
                };

                UserId::Id(user.osu_id)
            }
        };

        // 2. Try to fetch user
        let user_api = ctx.osu_api.get_user(user.clone(), None).await?;

        // 3. Try to find beatmap
        let beatmap_id = match &self.beatmap {
            Some(v) => {
                // Try to parse from regex
                let Some(beatmap_id) = parse_beatmap_link(v.as_ref()) else {
                    let builder = MessageBuilder::new().content(
                        "Failed to parse beatmap id from provided link",
                    );
                    cmd.update(ctx, &builder).await?;
                    return Ok(());
                };

                // TODO avoid converting but whatever for now
                beatmap_id as i64
            }
            None => 'blk: {
                // Try to fetch from recent messages
                let msgs = ctx
                    .http
                    .channel_messages(cmd.channel_id)
                    .limit(50)?
                    .await?
                    .models()
                    .await?;

                for msg in msgs {
                    if let Some(link) = find_beatmap_link(&msg) {
                        if let Some(bid) = parse_beatmap_link(link.as_ref()) {
                            break 'blk bid as i64;
                        }
                    }
                }

                let builder = MessageBuilder::new()
                    .content("Failed to find beatmap from recent 50 messages");
                cmd.update(ctx, &builder).await?;
                return Ok(());
            }
        };

        let user_id = match (&user, &user_api) {
            (UserId::Username(_), None) => {
                let builder = MessageBuilder::new()
                    .content("Provided username is not found on osu!");
                cmd.update(ctx, &builder).await?;
                return Ok(());
            }
            (UserId::Username(_), Some(api)) => api.id,
            (UserId::Id(id), None) => *id,
            (UserId::Id(_), Some(api)) => api.id,
        };

        let (scores, beatmap) = tokio::join!(
            ctx.db.select_beatmap_scores_by_user(
                beatmap_id,
                user_id,
                self.kind.is_tournament()
            ),
            ctx.osu_api.get_beatmap(beatmap_id)
        );

        let (mut scores, beatmap) = (scores?, beatmap?);

        if scores.is_empty() {
            let builder = MessageBuilder::new().content("No scores found :c");
            cmd.update(ctx, &builder).await?;
            return Ok(());
        }

        scores.sort_by(|a, b| b.score.cmp(&a.score));

        let embed = create_embed(&scores, &beatmap, &user_api, user_id)?;

        let builder = MessageBuilder::new().embed(embed);

        cmd.update(ctx, &builder).await?;

        Ok(())
    }
}

fn create_embed(
    scores: &[OsuDbMatchScore],
    beatmap: &OsuBeatmap,
    user_api: &Option<OsuUserExtended>,
    user_id: i64,
) -> Result<Embed> {
    let username = if let Some(user_api) = user_api {
        format!("{}", user_api.username)
    } else {
        format!("{}", user_id)
    };

    let author = EmbedAuthorBuilder::new(format!("Scores for {}", username));

    let first_score = &scores[0];
    let first_score_mods: OsuMods =
        OsuMods::from_bits_truncate(first_score.mods as u32);

    let mut description_text = String::with_capacity(200);

    let title = format!(
        "{} - {}[{}]",
        beatmap.beatmapset.artist, beatmap.beatmapset.title, beatmap.version,
    );

    let _ = writeln!(
        description_text,
        "**+{}** • **{}** • **{:.2}%** ",
        first_score_mods,
        first_score.score.to_formatted_string(&Locale::en),
        first_score.accuracy * 100.0,
    );

    let _ = write!(
        description_text,
        "~~**{:.2}pp**~~",
        first_score.pp.unwrap_or(0.0),
    );

    let _ = writeln!(
        description_text,
        " • <t:{}:R>",
        first_score.end_time.and_utc().timestamp()
    );

    let _ = writeln!(
        description_text,
        "[{}/{}/{}/{}] • {}/{}",
        first_score.count300,
        first_score.count100,
        first_score.count50,
        first_score.countmiss,
        first_score.max_combo,
        beatmap.max_combo.unwrap_or(0)
    );

    let _ = writeln!(
        description_text,
        "[**{}**](https://osu.ppy.sh/community/matches/{})",
        first_score.match_name, first_score.match_id
    );

    if scores.len() > 1 {
        let _ = writeln!(description_text, "");

        let _ = writeln!(description_text, "**__Other scores:__**");
    }

    for (idx, score) in scores.iter().enumerate().skip(1) {
        let _ = writeln!(
            description_text,
            "**{}**. {} • ~~{:.2}pp~~ • {:.2}% • +{}",
            idx + 1,
            score.score.to_formatted_string(&Locale::en),
            score.pp.unwrap_or(0.0),
            score.accuracy * 100.0,
            OsuMods::from_bits_truncate(score.mods as u32),
        );

        let _ = writeln!(
            description_text,
            "[**{}**](https://osu.ppy.sh/community/matches/{})",
            score.match_name, score.match_id
        );
    }

    let thumb_url =
        format!("https://b.ppy.sh/thumb/{}l.jpg", beatmap.beatmapset_id);

    Ok(EmbedBuilder::new()
        .color(865846)
        .description(description_text)
        .author(author)
        .thumbnail(ImageSource::url(thumb_url)?)
        .title(title)
        .url(format!("https://osu.ppy.sh/b/{}", beatmap.id))
        .build())
}
