use crate::{
    components::listing::ListingTrait,
    fumo_context::FumoContext,
    utils::{
        interaction::{InteractionCommand, InteractionComponent},
        searching::{find_beatmap_link, parse_beatmap_link},
        static_components::pages_components,
    },
};
use fumo_macro::listing;
use fumo_twilight::message::MessageBuilder;
use metrics::counter;
use osu_api::{
    models::{OsuBeatmap, RankStatus, osu_leaderboard::OsuLeaderboardLazer},
};

use twilight_interactions::command::{
    CommandModel, CommandOption, CreateCommand, CreateOption,
};
use twilight_model::{
    application::interaction::{Interaction, InteractionData},
    channel::message::embed::EmbedFooter,
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
    legacy: Option<bool>,
}

impl LeaderboardCommand {
    pub async fn handle(
        ctx: &FumoContext,
        cmd: InteractionCommand,
    ) -> Result<()> {
        let command = Self::from_interaction(cmd.data.clone().into())?;

        counter!("commands", "kind" => "country_leaderboard").increment(1);

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
                    &cmd,
                )
                .await; // TODO another cloning....................
            } else {
                let builder =
                    MessageBuilder::new().content("Please provide valid link");

                cmd.update(ctx, &builder).await?;
                return Ok(());
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
                    return country_leaderboard(
                        ctx,
                        bid,
                        self.mods.clone(),
                        self.sorting,
                        self.legacy,
                        &cmd,
                    )
                    .await;
                }
            }
        }

        let builder =
            MessageBuilder::new().content("Couldn't find any score/beatmap!");
        cmd.update(ctx, &builder).await?;
        Ok(())
    }
}

#[listing]
struct LeaderboardListing {
    scores: OsuLeaderboardLazer,
    beatmap: OsuBeatmap,
    user_position: Option<usize>,
    is_legacy: bool,
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
        let mut text = format!("Page {}/{}", self.current_page, self.max_pages);

        if let Some(pos) = self.user_position {
            text.push_str(&format!(
                " • Your position: {}/{}",
                pos,
                self.scores.scores.len()
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
            .scores
            .iter()
            .skip(start_at)
            .take(self.entries_per_page);

        for (index, score) in scores_iter.enumerate() {
            let mods_string = format!("{}", score.mods);

            let mut score_row = String::with_capacity(100);

            let _ = write!(
                score_row,
                "{}. [{}](https://osu.ppy.sh/u/{}) +**{}",
                index + 1 + start_at,
                score.user.as_ref().and_then(|u| Some(u.username.as_ref())).unwrap_or("NO USER"),
                score.user_id,
                mods_string
            );

            let mode_with_speed_change = score
                .mods
                .speed_changes();

            if let Some(speed) = mode_with_speed_change {
                let _ = write!(score_row, " (x{})**", speed);
            } else {
                let _ = write!(score_row, "**");
            };

            let _ = writeln!(description, "{}", score_row);

            let pp = match self.beatmap.status {
                RankStatus::Loved => "\\❤️".to_owned(),
                _ => format!("{:.2}pp", score.pp.unwrap_or(0.0)),
            };

            let osu_score = if self.is_legacy {
                score.legacy_total_score.to_formatted_string(&Locale::en)
            } else {
                score.total_score.to_formatted_string(&Locale::en)
            };

            let _ = writeln!(
                description,
                "{} • {:.2}% • {} • {}",
                score.rank.to_emoji(),
                score.accuracy * 100.0,
                pp,
                osu_score
            );

            let _ = writeln!(
                description,
                "[{}x/{}x] [{}/{}/{}/{}]",
                score.max_combo,
                self.beatmap.max_combo.unwrap_or(0),
                score.stats.great.unwrap_or(0),
                score.stats.ok.unwrap_or(0),
                score.stats.meh.unwrap_or(0),
                score.stats.miss.unwrap_or(0),
            );

            let _ = writeln!(description, "<t:{}:R>", score.ended_at.timestamp());
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
        //ctx.osu_api.get_countryleaderboard_fallback(bid, mods),
        ctx.osu_api.get_leaderboard_hidden(bid, true),
        ctx.osu_api.get_beatmap(bid as i64),
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

    let total_scores = clb.scores.len();

    match sorting {
        Some(LeaderboardSortingKind::Pp) => {
            if b.status != RankStatus::Loved {
                clb.scores.sort_by(|a, b| {
                    b
                        .pp
                        .partial_cmp(&a.pp)
                        .unwrap_or(Ordering::Equal)
                });
            }
        }
        Some(LeaderboardSortingKind::Score) => {
            clb.scores.sort_by(|a, b| match legacy {
                Some(true) => b.legacy_total_score.cmp(&a.legacy_total_score),
                None | Some(false) => {
                    b.total_score.cmp(&a.total_score)
                }
            });
        }
        None => {
            clb.scores.sort_by(|a, b| match legacy {
                Some(true) => b.legacy_total_score.cmp(&a.legacy_total_score),
                None | Some(false) => {
                    b.total_score.cmp(&a.total_score)
                }
            });
        }
    };

    let user_position: Option<usize> = match osu_user {
        Some(osu_user) => {
            let pos = clb
                .scores
                .iter()
                .enumerate()
                .find(|(_index, score)| score.user_id == osu_user.osu_id);

            if let Some((index, _score)) = pos {
                Some(index + 1)
            } else {
                None
            }
        }
        None => None,
    };

    let mut lb_list =
        LeaderboardListing::new(clb, b, user_position, legacy.unwrap_or(false))
            .calculate_pages(total_scores, 10);

    lb_list.update();

    let mut msg_builder = MessageBuilder::new()
        .embed(
            lb_list
                .embed
                .as_ref()
                .expect("embed should be present")
                .clone(),
        )
        .components(pages_components());

    let msg = cmd.update(ctx, &msg_builder).await?.model().await?;
    let msg_stream = component_stream!(ctx, msg);

    tokio::pin!(msg_stream);

    while let Some(Ok(component)) = msg_stream.next().await {
        lb_list.handle_interaction_component(ctx, &component).await;
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

    counter!("commands", "kind" => "country_leaderboard", "cmd" => "interaction").increment(1);

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
                return country_leaderboard(
                    ctx, bid, None, None, None, &command,
                )
                .await;
            }
        }
    }

    // If we didn't find anything
    builder = builder.content("Couldn't find any score/beatmap!");
    command.update(ctx, &builder).await?;
    Ok(())
}
