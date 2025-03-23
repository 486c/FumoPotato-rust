use crate::{
    components::listing::ListingTrait, fumo_context::FumoContext, utils::{
        interaction::{InteractionCommand, InteractionComponent}, searching::{find_beatmap_link, parse_beatmap_link}, static_components::pages_components, OSU_MAP_ID_NEW, OSU_MAP_ID_OLD
    }
};
use fumo_macro::listing;
use fumo_twilight::message::MessageBuilder;
use osu_api::{fallback_models::FallbackBeatmapScores, models::{OsuBeatmap, RankStatus}};

use twilight_interactions::command::{CommandModel, CommandOption, CreateCommand, CreateOption};
use twilight_model::{
    application::interaction::{Interaction, InteractionData},
    channel::message::{
        embed::EmbedFooter,
        Message,
    },
};
use twilight_util::builder::embed::{
    image_source::ImageSource, EmbedAuthorBuilder, EmbedBuilder,
};

use num_format::{Locale, ToFormattedString};

use tokio_stream::StreamExt;

use std::{cmp::Ordering, fmt::Write, time::Duration};

use eyre::Result;

#[derive(Debug, CommandOption, CreateOption, Copy, Clone)]
pub enum LeaderboardSortingKind {
    #[option(name = "Score", value = "score")]
    Score = 0,
    #[option(name = "Pp", value = "pp")]
    Pp = 1,
}

/// Country leaderboard
#[derive(CommandModel, CreateCommand, Debug)]
#[command(name = "leaderboard")]
pub struct LeaderboardCommand {
    /// Direct link to the beatmap
    #[command(min_length = 1, max_length = 256)]
    link: Option<String>,

    /// Mods
    #[command(min_length = 2, max_length = 100)]
    mods: Option<String>,

    /// Leaderboard sorting options
    sorting: Option<LeaderboardSortingKind>,

    /// Enable legacy scoring
    legacy: Option<bool>
}

impl LeaderboardCommand {
    pub async fn handle(
        ctx: &FumoContext,
        cmd: InteractionCommand,
    ) -> Result<()> {
        let command = Self::from_interaction(cmd.data.clone().into())?;

        ctx.stats.bot.cmd.with_label_values(&["leaderboard"]).inc();

        command.run(ctx, cmd).await
    }

    pub async fn run(
        &self,
        ctx: &FumoContext,
        cmd: InteractionCommand,
    ) -> Result<()> {
        cmd.defer(ctx).await?;

        // If link already provided go straight to parsing
        if let Some(link) = &self.link {
            if let Some(beatmap_id) = parse_beatmap_link(link) {
                return country_leaderboard(
                    ctx, 
                    beatmap_id, 
                    self.mods.clone(), 
                    self.sorting, 
                    self.legacy,
                    &cmd
                ).await; // TODO another cloning....................
            } else {
                let builder = MessageBuilder::new()
                    .content("Please provide valid link");

                cmd.update(ctx, &builder).await?;
                return Ok(())
            }
        }

        // If not try to search through recent messages
        let msgs = ctx
            .http
            .channel_messages(cmd.channel_id)
            .limit(50)?
            .await?
            .models()
            .await?;

        for m in msgs {
            if let Some(link) = find_beatmap_link(&m) {
                if let Some(bid) = parse_beatmap_link(link.as_ref()) {
                    return country_leaderboard(ctx, bid, self.mods.clone(), self.sorting, self.legacy, &cmd).await;
                }
            }
        }
        
        let builder = MessageBuilder::new().content("Couldn't find any score/beatmap!");
        cmd.update(ctx, &builder).await?;
        Ok(())
    }
}

#[listing]
struct LeaderboardListing {
    scores: FallbackBeatmapScores,
    beatmap: OsuBeatmap,
    user_position: Option<usize>,
    is_legacy: bool
}

impl ListingTrait for LeaderboardListing {
    async fn handle_interaction_component(
        &mut self,
        ctx: &FumoContext,
        component: &InteractionComponent,
    ) {
        let _ = component.defer(ctx).await;

        if let Some(data) = &component.data {
            match data.custom_id.as_ref() {
                "B1" => self.previous_page(),
                "B2" => self.next_page(),
                _ => {}
            }
        }
    }

    fn update(&mut self) {
        let mut text = format!(
            "Page {}/{}", 
            self.current_page, 
            self.max_pages
        );
        
        if let Some(pos) = self.user_position {
            text.push_str(&format!(
                " • Your position: {}/{}",
                pos,
                self.scores.items.len()
            ));
        }

        let footer = EmbedFooter {
            text,
            icon_url: None,
            proxy_icon_url: None,
        };

        let mut description = String::with_capacity(1500);

        let start_at = (self.current_page - 1) * self.entries_per_page;

        let scores_iter = self
            .scores
            .items
            .iter()
            .skip(start_at)
            .take(self.entries_per_page);

        for (index, score) in scores_iter.enumerate() {
            let mut mods_string = String::new();

            if score.stats.mods.difficulty.is_empty() {
                mods_string.push_str("NM");
            } else {
                score.stats.mods.difficulty
                    .iter()
                    .for_each(|x| mods_string.push_str(&x.acronym))
            };


            let mut score_row = String::with_capacity(100);

            let _ = write!(
                score_row,
                "{}. [{}](https://osu.ppy.sh/u/{}) +**{}",
                index + 1 + start_at,
                score.player.username,
                score.player.id,
                mods_string
            );

            let mode_with_speed_change = score.stats.mods.difficulty
                .iter()
                .find(|x| x.speed.is_some());

            if let Some(mode) = mode_with_speed_change {
                match mode.speed {
                    Some(speed) => {
                        let _ = write!(score_row, " (x{})**", speed);
                    },
                    None => {
                        let _ = write!(score_row, "**");
                    },
                }
            } else {
                let _ = write!(score_row, "**");
            };

            let _ = writeln!(description, "{}", score_row);

            let pp = match self.beatmap.status {
                RankStatus::Loved => "\\❤️".to_owned(),
                _ => format!("{:.2}pp", score.stats.performance),
            };

            let osu_score = if self.is_legacy {
                score.stats.score.legacy.to_formatted_string(&Locale::en)
            } else {
                score.stats.score.lazer.to_formatted_string(&Locale::en)
            };

            let _ = writeln!(
                description,
                "{} • {:.2}% • {} • {}",
                score.stats.rank.to_emoji(),
                score.stats.accuracy,
                pp,
                osu_score
            );

            let _ = writeln!(
                description,
                "[{}x/{}x] [{}/{}/{}/{}]",
                score.stats.combo,
                self.beatmap.max_combo.unwrap_or(0),
                score.counts.x300,
                score.counts.x100,
                score.counts.x50,
                score.counts.xmiss,
            );

            let _ = writeln!(description, "<t:{}:R>", score.date.timestamp());
        }

        let author = EmbedAuthorBuilder::new(self.beatmap.metadata())
            .url(format!("https://osu.ppy.sh/b/{}", self.beatmap.id))
            .build();

        let embed = EmbedBuilder::new()
            .color(865846)
            .author(author)
            .thumbnail(
                ImageSource::url(format!(
                    "https://assets.ppy.sh/beatmaps/{}/covers/list.jpg",
                    self.beatmap.beatmapset_id
                ))
                .unwrap(),
            )
            .description(description)
            .footer(footer)
            .build();

        self.embed = Some(embed);
    }
}

pub async fn country_leaderboard(
    ctx: &FumoContext,
    bid: i32,
    mods: Option<String>,
    sorting: Option<LeaderboardSortingKind>,
    legacy: Option<bool>,
    cmd: &InteractionCommand,
) -> Result<()> {
    let mut builder = MessageBuilder::new();

    let osu_user = osu_user!(ctx, cmd);

    let (clb_res, b_res) = tokio::join!(
        ctx.osu_api.get_countryleaderboard_fallback(bid, mods),
        ctx.osu_api.get_beatmap(bid),
    );

    let mut clb = match clb_res {
        Ok(lb) => lb,
        Err(e) => {
            builder =
                builder.content("Issues with leaderboard api. blame seneal");
            cmd.update(ctx, &builder).await?;
            return Err(e.into());
        }
    };

    let b = match b_res {
        Ok(b) => b,
        Err(e) => {
            builder = builder.content("Issues with osu!api. blame peppy");
            cmd.update(ctx, &builder).await?;
            return Err(eyre::Report::new(e));
        }
    };

    let total_scores = clb.items.len();

    match sorting {
        Some(LeaderboardSortingKind::Pp) => {
            if b.status != RankStatus::Loved {
                clb.items.sort_by(|a, b| b.stats.performance.partial_cmp(&a.stats.performance).unwrap_or(Ordering::Equal));
            }
        },
        Some(LeaderboardSortingKind::Score) => {
            clb.items.sort_by(|a, b| {

                match legacy {
                    Some(true) => {
                        b.stats.score.legacy.cmp(&a.stats.score.legacy)
                    },
                    None | Some(false) => {
                        b.stats.score.lazer.cmp(&a.stats.score.lazer)
                    },
                }
            });
        },
        None => {
            clb.items.sort_by(|a, b| {
                match legacy {
                    Some(true) => {
                        b.stats.score.legacy.cmp(&a.stats.score.legacy)
                    },
                    None | Some(false) => {
                        b.stats.score.lazer.cmp(&a.stats.score.lazer)
                    },
                }
            });
        },
    };

    let user_position: Option<usize> = match osu_user {
        Some(osu_user) => {
            let pos = clb.items
                .iter()
                .enumerate()
                .find(|(_index, score)| score.player.id == osu_user.osu_id);

            if let Some((index, _score)) = pos {
                Some(index + 1)
            } else {
                None
            }
        },
        None => None,
    };

    let mut lb_list = LeaderboardListing::new(clb, b, user_position, legacy.unwrap_or(false))
        .calculate_pages(total_scores, 10);

    lb_list.update();

    let mut msg_builder = MessageBuilder::new()
        .embed(
            lb_list
            .embed
            .as_ref()
            .expect("embed should be present")
            .clone()
        )
        .components(pages_components());

    let msg = cmd.update(ctx, &msg_builder).await?.model().await?;
    let msg_stream = component_stream!(ctx, msg);

    tokio::pin!(msg_stream);

    while let Some(Ok(component)) = msg_stream.next().await {
        lb_list
            .handle_interaction_component(ctx, &component)
            .await;
        lb_list.update();

        msg_builder = msg_builder.embed(
            lb_list
            .embed
            .as_ref()
            .expect("embed should be present")
            .clone(),
        );

        cmd.update(ctx, &msg_builder).await?;
    }

    // Clearing components
    msg_builder.clear_components();
    cmd.update(ctx, &msg_builder).await?;

    Ok(())
}

pub async fn run(ctx: &FumoContext, command: InteractionCommand) -> Result<()> {
    command.defer(ctx).await?;

    let mut builder = MessageBuilder::new();

    ctx.stats.bot.cmd.with_label_values(&["leaderboard_app_interaction"]).inc();

    // If we got app interaction
    if let Some(id) = command.data.target_id {
        let msg = ctx
            .http
            .message(command.channel_id, id.cast())
            .await?
            .model()
            .await?;

        if let Some(link) = find_beatmap_link(&msg) {
            if let Some(bid) = parse_beatmap_link(link.as_ref()) {
                return country_leaderboard(ctx, bid, None, None, None, &command).await;
            }
        }
    }

    // If we didn't find anything
    builder = builder.content("Couldn't find any score/beatmap!");
    command.update(ctx, &builder).await?;
    Ok(())
}
