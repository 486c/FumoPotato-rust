use fumo_twilight::message::MessageBuilder;
use rosu_pp::{
    model::mods::rosu_mods::{GameMod, GameMods},
    Performance,
};
use std::{collections::HashMap, sync::Arc, time::Duration};
use twilight_util::builder::embed::EmbedAuthorBuilder;

use num_format::{Locale, ToFormattedString};
use tokio_stream::StreamExt;

use std::fmt::Write;

use crate::{
    fumo_context::FumoContext,
    utils::{
        calc_ar, calc_od,
        interaction::{InteractionCommand, InteractionComponent},
        static_components::pages_components,
    },
};
use eyre::Result;
use osu_api::{
    error::OsuApiError,
    models::{
        osu_leaderboard::OsuScoreLazer, GetRanking, GetUserScores, OsuBeatmap,
        OsuBeatmapAttributesContainer, OsuGameMode, OsuUserExtended,
        RankingFilter, RankingKind, ScoresType, UserId,
    },
};
use twilight_interactions::command::{CommandModel, CreateCommand};
use twilight_model::{
    application::interaction::{Interaction, InteractionData},
    channel::message::{Embed, MessageFlags},
    id::Id,
};
use twilight_util::builder::embed::{
    EmbedBuilder, EmbedFooterBuilder, ImageSource,
};

const OSU_TRACKING_INTERVAL: Duration = Duration::from_secs(60);
const OSU_TRACKING_BATCH_SIZE: usize = 850;

fn create_tracking_embed(
    score: &OsuScoreLazer,
    user: &OsuUserExtended,
    beatmap: &OsuBeatmap,
    beatmap_attrs: &OsuBeatmapAttributesContainer,
    top_score_pos: Option<usize>,
) -> eyre::Result<Embed> {
    let mut description_text = String::with_capacity(100);

    let _ = write!(
        description_text,
        "{} [**{} - {} [{}]**](https://osu.ppy.sh/b/{}) ",
        score.ruleset_id.to_emoji(),
        beatmap.beatmapset.artist,
        beatmap.beatmapset.title,
        beatmap.version,
        beatmap.id
    );

    let (star_rating, max_combo) =
        (beatmap_attrs.star_rating, beatmap.max_combo.unwrap_or(0));

    let _ = writeln!(description_text, "[{:.2}★]", star_rating);

    if let Some(top_score_position) = top_score_pos {
        let _ = writeln!(
            description_text,
            ":trophy: __**New top score #{}**__",
            top_score_position
        );
    }

    let mut mods_string = String::with_capacity(24);
    if !score.mods.mods.is_empty() {
        let _ = write!(mods_string, "{}", &score.mods);

        if let Some(speed_change) = score.mods.speed_changes() {
            let _ = write!(mods_string, " (x{})", speed_change);
        }
    } else {
        let _ = write!(mods_string, "NM");
    }

    let _ = writeln!(
        description_text,
        "**{} • +{} • {} • {:.2}%**",
        score.rank.to_emoji(),
        &mods_string,
        score.total_score.to_formatted_string(&Locale::en),
        score.accuracy * 100.0
    );

    let _ = writeln!(
        description_text,
        "**{:.2}pp** • <t:{}:R>",
        score.pp.unwrap_or(0.0),
        score.ended_at.timestamp()
    );

    match score.ruleset_id {
        OsuGameMode::Mania => {
            let x320 = score.stats.perfect.unwrap_or(0);
            let x300 = score.stats.great.unwrap_or(0);
            let x200 = score.stats.good.unwrap_or(0);

            let ma_ratio = x320 as f32 / x300 as f32;
            let pa_ratio = x300 as f32 / x200 as f32;

            let _ = writeln!(
                description_text,
                "[{}/{}/{}/{}/{}/{}] • x{}/{} • MA: {:.2} PA: {:.2}",
                score.stats.perfect.unwrap_or(0),
                score.stats.great.unwrap_or(0),
                score.stats.good.unwrap_or(0),
                score.stats.ok.unwrap_or(0),
                score.stats.meh.unwrap_or(0),
                score.stats.miss.unwrap_or(0),
                score.max_combo,
                max_combo,
                ma_ratio,
                pa_ratio
            );
        }
        _ => {
            let _ = writeln!(
                description_text,
                "[{}/{}/{}/{}] • x{}/{}",
                score.stats.great.unwrap_or(0),
                score.stats.ok.unwrap_or(0),
                score.stats.meh.unwrap_or(0),
                score.stats.miss.unwrap_or(0),
                score.max_combo,
                max_combo
            );
        }
    }

    let bpm = beatmap.bpm.map(|x| {
        if let Some(speed_change) = score.mods.speed_changes() {
            return x * speed_change;
        };

        if score.mods.contains("DT") {
            return x * 1.5;
        }

        if score.mods.contains("HT") {
            return x * 0.75;
        }

        x
    });

    let beatmap_ar = beatmap.ar.ok_or(eyre::eyre!("beatmap ar is empty"))?;

    let beatmap_od =
        beatmap.accuracy.ok_or(eyre::eyre!("beatmap od is empty"))?;

    let beatmap_cs = beatmap.cs.ok_or(eyre::eyre!("beatmap cs is empty"))?;

    let beatmap_hp = beatmap.drain.ok_or(eyre::eyre!("beatmap hp is empty"))?;

    // AR
    let approach_rate = calc_ar(beatmap_ar, &score.mods);

    // OD
    let overall_difficulty =
        calc_od(beatmap_od, &score.mods, &score.ruleset_id);

    let mut circle_size = beatmap_cs;

    if score.mods.contains("HR") {
        circle_size = (circle_size * 1.3).min(10.0);
    }

    if score.mods.contains("EZ") {
        circle_size /= 2.0;
    }

    let mut hp_drain = beatmap_hp;

    if score.mods.contains("EZ") {
        hp_drain /= 2.0;
    }

    if score.mods.contains("HR") {
        hp_drain = (hp_drain * 1.4).min(10.0);
    }

    match score.ruleset_id {
        OsuGameMode::Fruits => {
            let _ = write!(
                description_text,
                "`AR: {:.2} CS: {:.2} HP: {:.2}",
                approach_rate, circle_size, hp_drain
            );
        }
        OsuGameMode::Mania => {
            let _ = write!(
                description_text,
                "`OD: {:.2} HP: {:.2}",
                overall_difficulty, hp_drain
            );
        }
        OsuGameMode::Osu => {
            let _ = write!(
                description_text,
                "`AR: {:.2} OD: {:.2} CS: {:.2} HP: {:.2}",
                approach_rate, overall_difficulty, circle_size, hp_drain
            );
        }
        OsuGameMode::Taiko => {
            let _ = write!(
                description_text,
                "`OD: {:.2} HP: {:.2}",
                overall_difficulty, hp_drain
            );
        }
    }

    let _ = writeln!(description_text, " BPM: {:.2}`", bpm.unwrap_or(0.0));

    let thumb_url =
        format!("https://b.ppy.sh/thumb/{}l.jpg", beatmap.beatmapset_id);

    let footer = EmbedFooterBuilder::new(format!(
        "Mapper {}",
        beatmap.beatmapset.creator
    ));

    let author = EmbedAuthorBuilder::new(format!(
        "{}: {:.2}pp (#{})",
        &user.username,
        user.statistics.pp,
        user.statistics.global_rank.unwrap_or(0),
    ))
    .url(format!("https://osu.ppy.sh/u/{}", user.id));

    Ok(EmbedBuilder::new()
        .color(0xbd49ff)
        .description(description_text)
        .footer(footer)
        .author(author)
        .thumbnail(ImageSource::url(thumb_url).unwrap())
        .url(format!("https://osu.ppy.sh/u/{}", user.id))
        .build())
}

async fn osu_track_checker(
    ctx: &FumoContext,
    scores: &mut [OsuScoreLazer],
    top_scores_hash: &mut HashMap<(i64, OsuGameMode), f32>,
    buff: &mut [i64],
) -> Result<()> {
    let mut len = 0;

    scores.iter().for_each(|score| {
        buff[len] = score.user_id;
        len += 1;
    });

    let linked_channels = ctx
        .db
        .select_osu_tracking_users_channels(&buff[0..len])
        .await?;

    for score in scores.iter_mut() {
        if let Some(channels_to_notify) = linked_channels.get(&score.user_id) {
            let min_top_score = if let Some(min_top_score) =
                top_scores_hash.get(&(score.user_id, score.ruleset_id))
            {
                // Cache hit
                ctx.stats
                    .bot
                    .cache
                    .with_label_values(&["osu_tracking_top_scores_hash_hit"])
                    .inc();
                *min_top_score
            } else {
                // Cache miss
                ctx.stats
                    .bot
                    .cache
                    .with_label_values(&["osu_tracking_top_scores_hash_miss"])
                    .inc();

                let get_user_scores = GetUserScores {
                    user_id: score.user_id,
                    kind: ScoresType::Best,
                    include_fails: Some(false),
                    mode: Some(score.ruleset_id),
                    limit: Some(100),
                    offset: None,
                };

                let user_top_scores = match ctx
                    .osu_api
                    .get_user_scores(get_user_scores)
                    .await
                {
                    Ok(v) => v,
                    Err(e) => {
                        tracing::error!(
                            user_id = score.user_id,
                            "Failed to fetch user top scores inside tracking loop: {e}",
                        );

                        continue;
                    }
                };

                let min_top_score = user_top_scores
                    .last()
                    .map(|x| x.pp.unwrap_or(0.0))
                    .unwrap_or(0.0);

                top_scores_hash
                    .insert((score.user_id, score.ruleset_id), min_top_score);

                min_top_score
            };

            let osu_user = match ctx
                .osu_api
                .get_user(UserId::Id(score.user_id), Some(score.ruleset_id))
                .await?
            {
                Some(v) => v,
                None => {
                    tracing::error!(
                        user_id = score.user_id,
                        "Cannot fetch user from get_scores_batch fetch"
                    );
                    continue;
                }
            };

            // Score might come with null pp's
            // Such as dt rates or +RX
            'blk: {
                if score.pp.is_none() {
                    let beatmap_bytes =
                        ctx.osu_api.download_beatmap(score.beatmap_id).await?;

                    let beatmap = rosu_pp::Beatmap::from_bytes(&beatmap_bytes)?;

                    if let Err(err) = beatmap.check_suspicion() {
                        tracing::error!(
                            beatmap_id = score.beatmap_id,
                            "Attempt to calculate suspicious beatmap: {err}"
                        );

                        break 'blk;
                    };

                    let mut rosu_mods = GameMods::new();

                    for mode in &score.mods.mods {
                        rosu_mods.insert(GameMod::new(
                            mode.acronym.as_str(),
                            score.ruleset_id.as_u8().into(),
                        ));
                    }

                    let mut diff_attrs_builder =
                        rosu_pp::Difficulty::new().mods(rosu_mods);

                    if let Some(speed_changes) = score.mods.speed_changes() {
                        diff_attrs_builder =
                            diff_attrs_builder.clock_rate(speed_changes.into())
                    };

                    let diff_attrs = diff_attrs_builder.calculate(&beatmap);

                    let mut pp_builder = Performance::new(diff_attrs)
                        .accuracy((score.accuracy * 100.0).into())
                        .misses(score.stats.miss.unwrap_or(0))
                        .slider_end_hits(
                            score.stats.slider_tail_hit.unwrap_or(0),
                        )
                        .large_tick_hits(
                            score.stats.large_tick_hit.unwrap_or(0),
                        )
                        .small_tick_hits(
                            score.stats.small_tick_hit.unwrap_or(0),
                        )
                        .n300(score.stats.great.unwrap_or(0))
                        .n100(score.stats.ok.unwrap_or(0))
                        .n50(score.stats.meh.unwrap_or(0))
                        .n_katu(score.stats.good.unwrap_or(0))
                        .n_geki(score.stats.great.unwrap_or(0))
                        .combo(score.max_combo);

                    if let Some(speed_changes) = score.mods.speed_changes() {
                        pp_builder = pp_builder.clock_rate(speed_changes.into())
                    };

                    pp_builder = match pp_builder
                        .try_mode(score.ruleset_id.as_u8().into())
                    {
                        Ok(pp_builder) => pp_builder,
                        Err(_) => {
                            tracing::error!(
                                beatmap_id = score.beatmap_id,
                                score_id = score.id,
                                "Failed to convert perfomance builder for gamemode"
                            );
                            break 'blk;
                        }
                    };

                    let pp = pp_builder.calculate();

                    score.pp = Some(pp.pp() as f32);
                }
            }

            let user_top_scores = if score.pp.unwrap_or(0.0) > min_top_score {
                // Update cache
                ctx.stats
                    .bot
                    .cache
                    .with_label_values(&["osu_tracking_top_scores_hash_force"])
                    .inc();

                let get_user_scores = GetUserScores {
                    user_id: score.user_id,
                    kind: ScoresType::Best,
                    include_fails: Some(false),
                    mode: Some(score.ruleset_id),
                    limit: Some(100),
                    offset: None,
                };

                let scores = match ctx
                    .osu_api
                    .get_user_scores(get_user_scores)
                    .await
                {
                    Ok(v) => v,
                    Err(e) => {
                        tracing::error!(
                            user_id = score.user_id,
                            "Failed to fetch user top scores inside tracking loop: {e}",
                        );
                        continue;
                    }
                };

                let min_top_score =
                    scores.last().map(|x| x.pp.unwrap_or(0.0)).unwrap_or(0.0);

                top_scores_hash
                    .entry((score.user_id, score.ruleset_id))
                    .and_modify(|x| *x = min_top_score)
                    .or_insert(min_top_score);

                scores
            } else {
                continue;
            };

            let (osu_beatmap_res, osu_beatmap_attributes_res) = tokio::join!(
                ctx.osu_api.get_beatmap(score.beatmap_id),
                ctx.osu_api.get_beatmap_attributes(
                    score.beatmap_id,
                    Some(&score.mods)
                )
            );

            let osu_beatmap = osu_beatmap_res?;
            let osu_beatmap_attributes = match osu_beatmap_attributes_res {
                Ok(v) => v,
                Err(e) => {
                    tracing::error!(
                        "Failed to fetch beatmap attributes: {}",
                        e
                    );
                    return Err(e.into());
                }
            };

            let top_score_position = user_top_scores
                .iter()
                .enumerate()
                .find(|(_i, x)| {
                    if let Some(beatmap) = &x.beatmap {
                        beatmap.id == score.beatmap_id as i32
                            && x.created_at == score.ended_at
                            && x.pp == score.pp
                    } else {
                        false
                    }
                })
                .map(|(i, _x)| i + 1);

            let embed = create_tracking_embed(
                score,
                &osu_user,
                &osu_beatmap,
                &osu_beatmap_attributes.attributes,
                top_score_position,
            )?;

            let embeds = &[embed];

            if let Some(channels) = &channels_to_notify.1 {
                for discord_channel in channels {
                    let _ = ctx
                        .http
                        .create_message(Id::new(*discord_channel as u64))
                        .embeds(embeds)
                        .unwrap()
                        .await
                        .inspect_err(|err| {
                            tracing::error!("Error during creation of embed messages for tracking: {err}")
                        });
                }
            }
        }
    }

    Ok(())
}

pub async fn osu_tracking_worker(ctx: Arc<FumoContext>) {
    tracing::info!("Starting osu tracking worker!");

    let mut top_scores_hash: HashMap<(i64, OsuGameMode), f32> = HashMap::new(); // TODO

    let mut cursor = {
        let state_lock = ctx.state.lock().await;
        state_lock.osu_checker_last_cursor
    };

    let mut user_id_buffer = [0i64; 1000];

    // top - old
    // bottom - new
    loop {
        let mut batch = match ctx.osu_api.get_scores_batch(&cursor).await {
            Ok(v) => v,
            Err(e) => {
                if let OsuApiError::CursorTooOld = e {
                    tracing::warn!(
                        cursor = cursor,
                        "cursor is too old, incrementing by 1000"
                    );

                    cursor = cursor.and_then(|x| Some(x + 1000));

                    if let Some(mut cursor) = &mut cursor {
                        cursor += 1000;
                    };

                    continue;
                };

                tracing::error!(
                    cursor = cursor,
                    "Error happened during get_scores_batch inside tracking loop: {e}"
                );

                continue;
            }
        };

        if batch.scores.is_empty() {
            tokio::time::sleep(OSU_TRACKING_INTERVAL).await;
            continue;
        }

        if batch.scores.len() < OSU_TRACKING_BATCH_SIZE {
            tokio::time::sleep(OSU_TRACKING_INTERVAL).await;
            continue;
        }

        let current_newest_score_id =
            batch.scores.last().map(|x| x.id).unwrap_or(0);

        let res = osu_track_checker(
            &ctx,
            &mut batch.scores,
            &mut top_scores_hash,
            &mut user_id_buffer,
        )
        .await;

        if let Err(err) = res {
            tracing::error!(
                cursor = cursor,
                "Failed to run osu_track_checker: {err}"
            )
        }

        cursor = Some(current_newest_score_id);

        let mut state_lock = ctx.state.lock().await;
        state_lock.osu_checker_last_cursor = cursor;
        drop(state_lock);
    }
}

/// Osu tracking commands
#[derive(CommandModel, CreateCommand, Debug)]
#[command(name = "tracking")]
pub enum OsuTracking {
    #[command(name = "add")]
    Add(OsuTrackingAdd),
    #[command(name = "remove")]
    Remove(OsuTrackingRemove),
    #[command(name = "add-bulk")]
    AddBulk(OsuTrackingAddBulk),
    #[command(name = "remove-all")]
    RemoveAll(OsuTrackingRemoveAll),
    #[command(name = "list")]
    List(OsuTrackingList),
}

/// Remove osu user from tracking
#[derive(CommandModel, CreateCommand, Debug)]
#[command(name = "remove")]
pub struct OsuTrackingRemove {
    /// osu! username or user id
    user: String,
}

impl OsuTrackingRemove {
    pub async fn run(
        &self,
        ctx: &FumoContext,
        cmd: InteractionCommand,
    ) -> Result<()> {
        let channel_id = cmd.channel_id.get().try_into()?;

        let osu_user = ctx
            .osu_api
            .get_user(
                UserId::Username(self.user.clone()), // TODO avoid stupid clones
                None,
            )
            .await?;

        let mut msg = MessageBuilder::new().flags(MessageFlags::EPHEMERAL);

        if osu_user.is_none() {
            msg = msg.content("User not found!");
            cmd.response(ctx, &msg).await?;
            return Ok(());
        }

        let osu_user = osu_user.unwrap();

        let osu_tracked =
            ctx.db.select_osu_tracking(channel_id, osu_user.id).await?;

        match osu_tracked {
            Some(_) => {
                ctx.db.remove_osu_tracking(channel_id, osu_user.id).await?;

                msg = msg.content("Successfully removed user from tracking");

                cmd.response(ctx, &msg).await?;
            }
            None => {
                msg = msg.content(
                    "This user is not currently tracked on this channel!",
                );

                cmd.response(ctx, &msg).await?;
            }
        }

        Ok(())
    }
}

/// Add osu user to the tracking
#[derive(CommandModel, CreateCommand, Debug)]
#[command(name = "add")]
pub struct OsuTrackingAdd {
    /// osu! username or user id
    user: String,
}

impl OsuTrackingAdd {
    pub async fn run(
        &self,
        ctx: &FumoContext,
        cmd: InteractionCommand,
    ) -> Result<()> {
        let osu_user = ctx
            .osu_api
            .get_user(
                UserId::Username(self.user.clone()), // TODO avoid stupid clones
                None,
            )
            .await?;

        let channel_id = cmd.channel_id.get().try_into()?;

        let mut msg = MessageBuilder::new()
            .flags(MessageFlags::EPHEMERAL)
            .content("User not found!");

        match osu_user {
            Some(osu_user) => {
                // Check if user is already tracked
                let osu_tracked =
                    ctx.db.select_osu_tracking(channel_id, osu_user.id).await?;

                match osu_tracked {
                    Some(_) => {
                        msg = msg.content("User is already tracked");
                        cmd.response(ctx, &msg).await?;
                        Ok(())
                    }
                    None => {
                        add_osu_tracking_user!(ctx, &osu_user, channel_id);

                        msg = msg.content(
                            "Successfully added user to the tracking!",
                        );
                        cmd.response(ctx, &msg).await?;
                        Ok(())
                    }
                }
            }
            None => {
                msg = msg.content("User not found!");
                cmd.response(ctx, &msg).await?;

                Ok(())
            }
        }
    }
}

#[derive(CommandModel, CreateCommand, Debug)]
#[command(
    name = "add-bulk",
    desc = "
    Add multiple users to the tracking, either based on country 
    or global leaderboards
"
)]
pub struct OsuTrackingAddBulk {
    /// Amount of users to add
    #[command(min_value = 1, max_value = 50)]
    amount: i64,

    /// Country code, if not specified then global leaderboard
    /// is going to be used
    #[command(min_length = 2, max_length = 2)]
    country: Option<String>,

    /// Starting page (1 page = 50 players)
    #[command(min_value = 1, max_value = 200)]
    page: Option<i64>,
}

impl OsuTrackingAddBulk {
    pub async fn run(
        &self,
        ctx: &FumoContext,
        cmd: InteractionCommand,
    ) -> Result<()> {
        let channel_id = cmd.channel_id.get().try_into()?;

        ctx.db.add_discord_channel(channel_id).await?;

        // Fetch all tracked users in current channel
        let tracked_users =
            ctx.db.select_osu_tracking_by_channel(channel_id).await?;

        let (ranking_kind, country_code) = match &self.country {
            Some(country_code) => {
                (RankingKind::Performance, Some(country_code.clone()))
            }
            None => (RankingKind::Performance, None),
        };

        let page = self.page.unwrap_or(0);

        let get_ranking = GetRanking {
            mode: OsuGameMode::Osu,
            kind: ranking_kind,
            filter: RankingFilter::All,
            country: country_code,
            page: Some(page as u32),
        };

        // Fetch users that should be added
        let rankings = ctx
            .osu_api
            .get_rankings(&get_ranking, self.amount as usize)
            .await?;

        let mut str = String::new();

        let _ = writeln!(str, "```");

        for stats in rankings.ranking {
            // TODO lmao wtf is this refactor ASAP
            if tracked_users.iter().any(|x| x.osu_id == stats.user.id) {
                let _ =
                    writeln!(str, "{} - Already tracked", stats.user.username);
            } else {
                ctx.db
                    .add_osu_player(stats.user.id, &stats.user.username)
                    .await?;

                ctx.db.add_osu_tracking(channel_id, stats.user.id).await?;

                let _ = writeln!(str, "{} - Added", stats.user.username);
            }
        }

        let _ = writeln!(str, "```");

        let msg = MessageBuilder::new()
            .flags(MessageFlags::EPHEMERAL)
            .content(str);

        cmd.response(ctx, &msg).await?;

        Ok(())
    }
}

#[derive(CommandModel, CreateCommand, Debug)]
#[command(
    name = "remove-all",
    desc = "
    Remove all tracked users from current channel
"
)]
pub struct OsuTrackingRemoveAll {}

impl OsuTrackingRemoveAll {
    pub async fn run(
        &self,
        ctx: &FumoContext,
        cmd: InteractionCommand,
    ) -> Result<()> {
        let channel_id: i64 = cmd.channel_id.get().try_into()?;

        ctx.db.remove_all_osu_tracking(channel_id).await?;

        let msg = MessageBuilder::new()
            .flags(MessageFlags::EPHEMERAL)
            .content(
                "
                Successfully removed all tracked users from current channel",
            );

        cmd.response(ctx, &msg).await?;

        Ok(())
    }
}

#[derive(CommandModel, CreateCommand, Debug)]
#[command(
    name = "list",
    desc = "
    List all tracked users on current channel
"
)]
pub struct OsuTrackingList {}

impl OsuTrackingList {
    pub async fn run(
        &self,
        ctx: &FumoContext,
        cmd: InteractionCommand,
    ) -> Result<()> {
        cmd.defer(ctx).await?;

        let channel_id: i64 = cmd.channel_id.get().try_into()?;

        let tracked_users =
            ctx.db.select_osu_tracking_by_channel(channel_id).await?;

        let elem_per_page = 10;

        let pages: u32 =
            (tracked_users.len() as f32 / elem_per_page as f32).ceil() as u32;

        let mut current_page = 1;

        let footer_text = format!(
            "Tracked users: {} • Page: {}/{}",
            tracked_users.len(),
            current_page,
            pages
        );

        let mut body_text = String::with_capacity(100);

        for tracked_user in tracked_users.iter().take(elem_per_page as usize) {
            let _ = writeln!(body_text, "{}", &tracked_user.osu_username);
        }

        let embed = EmbedBuilder::new()
            .color(0xbd49ff)
            .title("Tracked users")
            .footer(EmbedFooterBuilder::new(footer_text))
            .description(body_text)
            .build();

        let mut msg_builder = MessageBuilder::new();

        msg_builder.embed = Some(embed);
        msg_builder.components = Some(pages_components());

        let msg = cmd.update(ctx, &msg_builder).await?.model().await?;

        let stream = component_stream!(ctx, msg);

        tokio::pin!(stream);

        while let Some(Ok(component)) = stream.next().await {
            if let Some(data) = &component.data {
                match data.custom_id.as_ref() {
                    "B1" => current_page = (current_page - 1).max(1),
                    "B2" => current_page = (current_page + 1).min(pages),
                    _ => {}
                }
            }

            let start_at = (current_page - 1) * elem_per_page;

            let embed = &mut msg_builder.embed;

            component.defer(ctx).await?;

            // Update body
            if let Some(embed) = embed {
                if let Some(description) = &mut embed.description {
                    description.clear();
                    for tracked_user in tracked_users
                        .iter()
                        .skip(start_at as usize)
                        .take(elem_per_page as usize)
                    {
                        let _ = writeln!(
                            description,
                            "{}",
                            &tracked_user.osu_username
                        );
                    }
                }

                if let Some(footer) = &mut embed.footer {
                    footer.text.clear();

                    let _ = write!(
                        footer.text,
                        "Tracked users: {} • Page: {}/{}",
                        tracked_users.len(),
                        current_page,
                        pages
                    );
                }
            }

            cmd.update(ctx, &msg_builder).await?;
        }

        if let Some(components) = &mut msg_builder.components {
            components.clear();
        }

        cmd.update(ctx, &msg_builder).await?;

        Ok(())
    }
}
