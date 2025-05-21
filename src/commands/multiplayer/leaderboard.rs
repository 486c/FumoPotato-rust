use fumo_database::osu::OsuDbMatchScore;
use num_format::Locale;
use num_format::ToFormattedString;
use osu_api::models::OsuBeatmap;
use osu_api::models::OsuMods;
use twilight_model::channel::message::Embed;
use twilight_util::builder::embed::ImageSource;
use std::collections::HashSet;
use std::fmt::Write;
use fumo_macro::listing;
use fumo_twilight::message::MessageBuilder;
use twilight_interactions::command::{CommandModel, CreateCommand};
use twilight_util::builder::embed::{EmbedBuilder, EmbedFooterBuilder};

use twilight_model::application::interaction::InteractionData;
use twilight_model::application::interaction::Interaction;
use tokio_stream::StreamExt;

use std::time::Duration;

use crate::{components::listing::ListingTrait, fumo_context::FumoContext, utils::{interaction::{ InteractionCommand, InteractionComponent}, searching::{find_beatmap_link, parse_beatmap_link}, static_components::pages_components}};
use eyre::Result;
use super::ListKind;

/// Show multiplayer leaderboard for beatmap
#[derive(CommandModel, CreateCommand, Debug)]
#[command(name = "leaderboard")]
pub struct MultiplayerLeaderboard {
    /// List all matches or only in tournament related
    pub kind: ListKind,

    /// Beatmap ID or beatmap link
    pub beatmap: Option<String>,
}


#[listing]
pub struct LeaderboardListing {
    beatmap: OsuBeatmap,
    scores: Vec<OsuDbMatchScore>,
}

impl ListingTrait for LeaderboardListing {
    async fn handle_interaction_component(
        &mut self,
        ctx: &FumoContext,
        component: &crate::utils::interaction::InteractionComponent,
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
        let footer = EmbedFooterBuilder::new(format!(
            "Total: {} • Page {}/{}",
            self.scores.len(),
            self.current_page,
            self.max_pages
        ));

        let start_at = (self.current_page - 1) * self.entries_per_page;

        let scores_iter = self
            .scores
            .iter()
            .enumerate()
            .skip(start_at)
            .take(self.entries_per_page);

        let mut description = String::with_capacity(100);

        for (idx, score) in scores_iter {
            let _ = writeln!(
                description,
                "{}. [{}](https://osu.ppy.sh/users/{}) • {} • {:.2}% • +{}",
                idx,
                score.osu_username.as_ref().map_or("Unknown", |v| v.as_str()),
                score.user_id,
                score.score.to_formatted_string(&Locale::en),
                score.accuracy * 100.0,
                OsuMods::from_bits_truncate(score.mods as u32)
            );

            let _ = writeln!(
                description,
                "[{}](https://osu.ppy.sh/community/matches/{})",
                score.match_name,
                score.match_id
            );
        }

        let thumb_url = format!("https://b.ppy.sh/thumb/{}l.jpg", self.beatmap.beatmapset_id);

        let embed = EmbedBuilder::new()
            .color(123432)
            .title(self.beatmap.metadata())
            .description(&description)
            .footer(footer)
            .thumbnail(ImageSource::url(thumb_url).expect("Thumbnail url should be valid"))
            .url(format!("https://osu.ppy.sh/b/{}", self.beatmap.id))
            .build();

        self.embed = Some(embed)
    }
}

impl MultiplayerLeaderboard {
    pub async fn run(
        &self,
        ctx: &FumoContext,
        cmd: InteractionCommand,
    ) -> Result<()> {
        cmd.defer(ctx).await?;

        let beatmap = match &self.beatmap {
            Some(link) => {
                let Some(beatmap_id) = parse_beatmap_link(link.as_ref()) else {
                    let builder = MessageBuilder::new()
                        .content("Failed to parse beatmap id from provided link");
                    cmd.update(ctx, &builder).await?;
                    return Ok(())
                };

                beatmap_id as i64
            },
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
                };

                let builder = MessageBuilder::new()
                    .content("Failed to find beatmap from recent 50 messages");
                cmd.update(ctx, &builder).await?;
                return Ok(());
            },
        };

        let mut scores = ctx.db.select_beatmap_scores(
            beatmap,
            self.kind.is_tournament()
        ).await?;

        // Finding a users that doesn't have a cached username
        let fetch_usernames: Vec<i64> = scores.iter()
            .filter(|score| score.osu_username.is_none())
            .map(|score| score.user_id)
            .collect::<HashSet<i64>>()
            .into_iter()
            .collect::<Vec<i64>>();

        if !fetch_usernames.is_empty() {
            let temp_msg = MessageBuilder::new()
                .embed(create_please_wait_embed(&scores, fetch_usernames.len()));

            cmd.update(ctx, &temp_msg).await?;

            let users_response = ctx.osu_api.lookup_users(&fetch_usernames).await?;

            // Filling usernames
            for user in users_response.users {
                ctx.db.insert_username(user.id, &user.username).await?;
            }

            // Fetching second time, TODO yep thats bad
            scores = ctx.db.select_beatmap_scores(
                beatmap,
                self.kind.is_tournament()
            ).await?;
        }

        let beatmap = ctx.osu_api.get_beatmap(beatmap as i32).await?;

        let scores_len = scores.len();

        scores.sort_by(|a, b| b.score.cmp(&a.score));

        let mut leaderboard_list = LeaderboardListing::new(beatmap, scores)
            .calculate_pages(scores_len, 10);

        leaderboard_list.update();

        let mut msg_builder = MessageBuilder::new()
            .embed(
                leaderboard_list
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
            leaderboard_list
                .handle_interaction_component(ctx, &component)
                .await;

            leaderboard_list.update();

            msg_builder = msg_builder
                .embed(
                    leaderboard_list
                    .embed
                    .as_ref()
                    .expect("embed should be present")
                    .clone(),
                )
                .components(pages_components());

            cmd.update(ctx, &msg_builder).await?.model().await?;
        }

        msg_builder.clear_components();
        cmd.update(ctx, &msg_builder).await?;

        Ok(())
    }
}

#[inline]
pub fn create_please_wait_embed(
    scores: &[OsuDbMatchScore],
    user_ids_to_fetch: usize,
) -> Embed {
    let mut description = String::with_capacity(30);
    
    let _ = writeln!(
        description, 
        "Found {} scores", scores.len()
    );

    let _ = writeln!(
        description, 
        "Fetching and caching {} usernames, please wait a bit UwU~",
        user_ids_to_fetch
    );

    EmbedBuilder::new()
        .color(123432)
        .description(description)
        .build()
}
