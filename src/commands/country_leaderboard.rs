use crate::{
    fumo_context::FumoContext,
    utils::{
        interaction::{InteractionCommand, InteractionComponent}, static_components::pages_components, OSU_MAP_ID_NEW, OSU_MAP_ID_OLD
    },
};
use fumo_database::osu::OsuDbUser;
use fumo_twilight::message::MessageBuilder;
use osu_api::models::{OsuBeatmap, OsuScore, RankStatus};

use twilight_model::{
    application::interaction::{Interaction, InteractionData},
    channel::message::{
        embed::{Embed, EmbedFooter},
        Message,
    },
};
use twilight_util::builder::embed::{
    image_source::ImageSource, EmbedAuthorBuilder, EmbedBuilder,
};

use num_format::{Locale, ToFormattedString};

use tokio_stream::StreamExt;

use std::{fmt::Write, time::Duration};

use eyre::Result;

struct LeaderboardListing {
    pages: i32,
    curr_page: i32,

    scores: Vec<OsuScore>,
    beatmap: OsuBeatmap,
    user_position: Option<usize>,

    embed: Embed,
    msg: MessageBuilder,
}

impl LeaderboardListing {
    fn new(
        user: Option<OsuDbUser>,
        scores: Vec<OsuScore>,
        beatmap: OsuBeatmap,
    ) -> LeaderboardListing {
        let pages: i32 =
            (scores.len() as f32 / 10.0).ceil().clamp(1.0, 20.0) as i32;

        let author = EmbedAuthorBuilder::new(beatmap.metadata())
            .url(format!("https://osu.ppy.sh/b/{}", beatmap.id))
            .build();

        let embed = EmbedBuilder::new()
            .author(author)
            .color(865846)
            .thumbnail(
                ImageSource::url(format!(
                    "https://assets.ppy.sh/beatmaps/{}/covers/list.jpg",
                    beatmap.beatmapset_id
                ))
                .unwrap(),
            )
            .build();

        let user_position: Option<usize> = match user {
            Some(user) => {
                let pos_score = scores
                    .iter()
                    .enumerate()
                    .find(|(_index, score)| score.user_id == user.osu_id);

                if let Some((index, _score)) = pos_score {
                    Some(index + 1)
                } else {
                    None
                }
            }
            None => None,
        };

        let msg = MessageBuilder::new().components(pages_components());

        let mut lb = LeaderboardListing {
            pages,
            curr_page: 1,
            scores,
            beatmap,
            user_position,
            embed,
            msg,
        };

        lb.update_state();

        lb
    }

    fn clear_components(&mut self) {
        if let Some(components) = &mut self.msg.components {
            components.clear()
        }
    }

    fn next_page(&mut self) {
        self.curr_page += 1;
        if self.curr_page > self.pages {
            self.curr_page = self.pages;
        }
    }

    fn prev_page(&mut self) {
        self.curr_page -= 1;
        if self.curr_page < 1 {
            self.curr_page = 1;
        }
    }

    fn update_state(&mut self) {
        let mut text = format!("Page {}/{}", self.curr_page, self.pages);

        if let Some(pos) = self.user_position {
            text.push_str(&format!(
                " • Your position: {}/{}",
                pos,
                self.scores.len()
            ));
        }

        self.embed.footer = Some(EmbedFooter {
            text,
            icon_url: None,
            proxy_icon_url: None,
        });

        let mut st = String::with_capacity(1500);

        let start_at = (self.curr_page - 1) * 10;
        for (index, score) in self
            .scores
            .iter()
            .skip(start_at as usize)
            .take(10)
            .enumerate()
        {
            let user_score = match &score.user {
                Some(user) => user,
                None => {
                    println!("Error: Got score without user info!");
                    return;
                }
            };

            let _ = writeln!(
                st,
                "{}. [{}](https://osu.ppy.sh/u/{}) +**{}**",
                index as i32 + 1 + start_at,
                user_score.username,
                user_score.id,
                score.mods
            );

            let pp = match self.beatmap.status {
                RankStatus::Loved => "\\❤️".to_owned(),
                _ => format!("{:.2}pp", score.pp.unwrap_or(0.0)),
            };

            let _ = writeln!(
                st,
                "{} • {:.2}% • {} • {}",
                score.rank.to_emoji(),
                score.accuracy * 100.0,
                pp,
                score.score.to_formatted_string(&Locale::en)
            );

            let _ = writeln!(
                st,
                "[{}x/{}x] [{}/{}/{}/{}]",
                score.max_combo.unwrap_or(0),
                self.beatmap.max_combo.unwrap_or(0),
                score.stats.count300.unwrap_or(0),
                score.stats.count100.unwrap_or(0),
                score.stats.count50.unwrap_or(0),
                score.stats.countmiss.unwrap_or(0),
            );

            let _ = writeln!(st, "<t:{}:R>", score.created_at.timestamp());
        }

        self.embed.description = Some(st);

        self.msg.embed = Some(self.embed.clone()); // TODO remove clone???
    }
}

fn find_link(msg: &Message) -> Option<&String> {
    match msg.author.id.get() {
        // owo bot
        289066747443675143 => msg.embeds.first()?.author.as_ref()?.url.as_ref(),
        // bath bot & mikaizuku
        297073686916366336 | 839937716921565252 => {
            msg.embeds.first()?.url.as_ref()
        }

        _ => None,
    }
}

fn parse_link(str: &str) -> Option<i32> {
    if !str.contains("https://osu.ppy.sh") {
        return None;
    }

    let m = if let Some(o) = OSU_MAP_ID_OLD.get().captures(str) {
        o.get(1)
    } else {
        OSU_MAP_ID_NEW.get().captures(str).and_then(|o| o.get(2))
    };

    m.and_then(|o| o.as_str().parse().ok())
}

pub async fn country_leaderboard(
    ctx: &FumoContext,
    bid: i32,
    command: &InteractionCommand,
) -> Result<()> {
    let mut builder = MessageBuilder::new();

    let osu_user = osu_user!(ctx, command);

    let clb = match ctx.osu_api.get_countryleaderboard_fallback(bid).await {
        Ok(lb) => lb,
        Err(e) => {
            builder =
                builder.content("Issues with leaderboard api. blame seneal");
            command.update(ctx, &builder).await?;
            return Err(e.into());
        }
    };

    let b = match ctx.osu_api.get_beatmap(bid).await {
        Ok(b) => b,
        Err(e) => {
            builder = builder.content("Issues with osu!api. blame peppy");
            command.update(ctx, &builder).await?;
            return Err(eyre::Report::new(e));
        }
    };

    let mut lb = LeaderboardListing::new(osu_user, clb.scores, b);

    builder = builder.embed(lb.embed.clone());
    builder = builder.components(pages_components()); // TODO

    let msg = command.update(ctx, &builder).await?.model().await?;

    let stream = component_stream!(ctx, msg);

    tokio::pin!(stream);

    while let Some(Ok(component)) = stream.next().await {
        if let Some(data) = &component.data {
            match data.custom_id.as_ref() {
                "B1" => lb.prev_page(),
                "B2" => lb.next_page(),
                _ => {}
            }
        }

        lb.update_state();

        component.defer(ctx).await?;
        command.update(ctx, &lb.msg).await?;
    }

    lb.clear_components();
    command.update(ctx, &lb.msg).await?;

    Ok(())
}

pub async fn run(ctx: &FumoContext, command: InteractionCommand) -> Result<()> {
    command.defer(ctx).await?;

    let mut builder = MessageBuilder::new();

    // If we got direct link
    if let Some(link) = command.get_option_string("link") {
        if let Some(bid) = parse_link(link) {
            // bid = v;
            return country_leaderboard(ctx, bid, &command).await;
        } else {
            builder = builder.content("Invalid link format!");
            command.update(ctx, &builder).await?;
            return Ok(());
        }
    }

    // If we got app interaction
    if let Some(id) = command.data.target_id {
        let msg = ctx
            .http
            .message(command.channel_id, id.cast())
            .await?
            .model()
            .await?;

        if let Some(link) = find_link(&msg) {
            if let Some(bid) = parse_link(link.as_ref()) {
                return country_leaderboard(ctx, bid, &command).await;
            }
        }
    }

    // If we got basic interaction
    let msgs = ctx
        .http
        .channel_messages(command.channel_id)
        .limit(50)?
        .await?
        .models()
        .await?;

    for m in msgs {
        if let Some(link) = find_link(&m) {
            if let Some(bid) = parse_link(link.as_ref()) {
                return country_leaderboard(ctx, bid, &command).await;
            }
        }
    }

    // If we didn't find anything
    builder = builder.content("Couldn't find any score/beatmap!");
    command.update(ctx, &builder).await?;
    Ok(())
}
