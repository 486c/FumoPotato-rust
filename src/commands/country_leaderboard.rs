use crate::osu_api::models::{ OsuBeatmap, OsuScore, RankStatus };
use crate::fumo_context::FumoContext;
use crate::utils::{ InteractionComponent, InteractionCommand, MessageBuilder };

use twilight_util::builder::embed::{ image_source::ImageSource, EmbedBuilder, EmbedAuthorBuilder };
use twilight_model::channel::message::component::{ Component, Button, ButtonStyle, ActionRow};
use twilight_model::channel::message::{ embed::{ Embed, EmbedFooter}, Message };
use twilight_model::application::interaction::{Interaction, InteractionData};

use num_format::{Locale, ToFormattedString};

use tokio_stream::StreamExt;

use std::fmt::Write;
use std::time::Duration;

use eyre::Result;

struct LeaderboardListing<'a> {
    pages: i32,
    curr_page: i32,

    scores: &'a Vec<OsuScore>,
    beatmap: &'a OsuBeatmap,

    embed: Embed,
}

impl<'a> LeaderboardListing<'a> {
    fn new(s: &'a Vec<OsuScore>, b: &'a OsuBeatmap) -> LeaderboardListing {
        let mut pages: i32 = (s.len() as f32 / 10.0 ).ceil() as i32;
        if pages == 0 {
            pages = 1;
        }

        let mut embed = EmbedBuilder::new();

        let author = EmbedAuthorBuilder::new(b.metadata())
            .url(format!("https://osu.ppy.sh/b/{}", b.id))
            .build();

        embed = embed
            .author(author)
            .thumbnail(
                ImageSource::url(format!(
                    "https://assets.ppy.sh/beatmaps/{}/covers/list.jpg",
                    b.beatmapset_id
                ))
                .unwrap()
            )
            .color(865846);

        let embed = embed.build();

        let mut lb = LeaderboardListing {
            pages,
            curr_page: 1,
            scores: s,
            beatmap: b,
            embed,
        };

        lb.update_embed();

        lb
    }

    fn components() -> Vec<Component> {
        let mut vec = Vec::with_capacity(2);

        let button = Component::Button( Button {
            custom_id: Some("B1".to_owned()),
            disabled: false,
            label: Some("Prev".to_owned()),
            style: ButtonStyle::Primary,
            url: None,
            emoji: None,
        }) ;
        vec.push(button);

        let button = Component::Button( Button {
            custom_id: Some("B2".to_owned()),
            disabled: false,
            label: Some("Next".to_owned()),
            style: ButtonStyle::Primary,
            url: None,
            emoji: None,
        }) ;
        vec.push(button);

        let component = Component::ActionRow(
            ActionRow {
                components: vec
            }
        );

        vec![component]
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

    fn update_embed(&mut self) {
        self.embed.footer = Some(
            EmbedFooter{
                text: format!("Page {}/{}", self.curr_page, self.pages),
                icon_url: None,
                proxy_icon_url: None,
            }
        );

        let mut st = String::with_capacity(1500);

        let start_at = (self.curr_page-1)*10;
        for (index, s) in self.scores.iter()
            .skip(start_at as usize)
            .take(10)
            .enumerate()
        {
            let _ = writeln!(st, "{}. [{}](https://osu.ppy.sh/u/{}) +**{}**",
                index as i32 + 1  + start_at, s.user.username, s.user.id, s.mods.to_string()
            );

            let pp: String = match self.beatmap.ranked {
                RankStatus::Loved => "\\❤️".to_owned(),
                _ => format!("{:.2}pp", s.pp.unwrap_or(0.0)),
            };

            let _ = writeln!(st, "{} • {:.2}% • {} • {}",
                s.rank.to_emoji(), s.accuracy * 100.0, pp,
                s.score.to_formatted_string(&Locale::en)
            );

            let _  = writeln!(st, "[{}x/{}x] [{}/{}/{}/{}]",
                s.max_combo, self.beatmap.max_combo,
                s.stats.count300, s.stats.count100, s.stats.count50,
                s.stats.countmiss
            );

            let _  = writeln!(st, "<t:{}:R>",
                s.created_at.timestamp()
            );
        }

        self.embed.description = Some(st);
    }
}

fn find_link(msg: &Message) -> Option<&String> {
    match msg.author.id.get() {
        // owo bot
        289066747443675143 => {
            msg.embeds.get(0)?.author.as_ref()?
                .url.as_ref()
        },
        // bath bot & mikaizuku
        297073686916366336 | 839937716921565252 => {
            msg.embeds.get(0)?.url.as_ref()
        }

        _ => None,
    }
}

fn parse_link(str: &str) -> Option<i32> {
    //TODO rewrite this shit xD
    let split: Vec<&str> = str.split('/').collect();

    // if full beatmapset link
    if split.len() == 6 {
        // Should never fail
        return Some(split.get(5).unwrap().parse::<i32>().unwrap());
    }

    // if compact link to beatmap
    // aka /b/id & /beatmaps/id
    if split.len() == 5 {
        // Also Should never fail
        return Some(split.get(4).unwrap().parse::<i32>().unwrap());
    }

    None
}

pub async fn run(ctx: &FumoContext, command: InteractionCommand) -> Result<()> {
    command.defer(ctx).await.unwrap();

    let mut builder = MessageBuilder::new();

    let mut bid: i32 = -1;
    
    // If we got direct link
    if let Some(link) = command.get_option_string("link") {
        if let Some(v) = parse_link(link) {
            bid = v;
        } else {
            builder = builder.content("Invalid link format!");
            command.update(ctx, &builder).await.unwrap();
            return Ok(());
        }
    }

    match command.data.target_id {
        // If we got app interaction
        Some(id) => {
            let msg = ctx.http.message(
                command.channel_id,
                id.cast()
            ).await.unwrap()
                .model().await.unwrap();
 
            if let Some(link) = find_link(&msg) {
                if let Some(id) = parse_link(link.as_ref()) {
                    bid = id;
                }
            }
        },

        // If we got basic interaction without direct link
        None => {
            let msgs = ctx.http.
                channel_messages(command.channel_id)
                .limit(50).unwrap()
                .await.unwrap()
                .models().await.unwrap();

            for m in msgs {
                if let Some(link) = find_link(&m) {
                    if let Some(id) = parse_link(link.as_ref()) {
                        bid = id;
                        break;
                    }
                }
            }
        }
    }

    // If bid is still -1 after all
    if bid == -1 {
        builder = builder.content("Couldn't find any score/beatmap!");
        command.update(ctx, &builder).await.unwrap();
        return Ok(());
    }

    let clb = match ctx.osu_api.get_countryleaderboard(bid).await {
        Some(lb) => lb,
        None => {
            builder = builder.content("Issues with leaderboard api. blame seneal");
            command.update(ctx, &builder).await.unwrap();
            return Ok(());
        }
    };

    let b = match ctx.osu_api.get_beatmap(bid).await {
        Some(b) => b,
        None => {
            builder = builder.content("Issues with osu!api. blame peppy");
            command.update(ctx, &builder).await.unwrap();
            return Ok(());
        }
    };

    let mut lb = LeaderboardListing::new(&clb.scores, &b);

    builder = builder.embed(lb.embed.clone());
    builder = builder.components(LeaderboardListing::components());

    let msg = command.update(ctx, &builder).await.unwrap()
        .model().await.unwrap();

    let stream = ctx.standby.wait_for_component_stream(msg.id, |_e: &Interaction| {
        true
    }) 
    .map(|event| {
        let Interaction {
            channel_id,
            data,
            guild_id,
            kind,
            id,
            token,
            ..
        } = event;

        if let Some(InteractionData::MessageComponent(data)) = data {
            InteractionComponent {
                channel_id,
                data: Some(data),
                kind,
                id,
                token,
                guild_id
            } 
        } else {
            InteractionComponent {
                channel_id,
                data: None,
                kind,
                id,
                token,
                guild_id
            } 
        }
    })
    .timeout(Duration::from_secs(20));

    tokio::pin!(stream);

    while let Some(Ok(component)) = stream.next().await {
        if let Some(data) = &component.data {
            match data.custom_id.as_ref() {
                "B1" => lb.prev_page(),
                "B2" => lb.next_page(),
                _ => {},
            }
        } 

        lb.update_embed();
        builder = builder.embed(lb.embed.clone()); // TODO remove cloning
        component.defer(ctx).await?;
        command.update(ctx, &builder).await?;
    }

    builder = builder.components(Vec::new());
    command.update(ctx, &builder).await?;

    Ok(())
}
