use crate::osu_api::models::{ OsuBeatmap, OsuScore, RankStatus };
use crate::fumo_context::FumoContext;
use crate::utils::{ InteractionCommand, MessageBuilder };
use twilight_model::channel::embed::{ Embed, EmbedFooter };

use twilight_util::builder::embed::{ EmbedBuilder, EmbedAuthorBuilder };
use twilight_util::builder::embed::image_source::ImageSource;
use num_format::{Locale, ToFormattedString};

use twilight_model::application::component::Component;
use twilight_model::application::component::button::Button;
use twilight_model::application::component::button::ButtonStyle;
use twilight_model::application::component::action_row::ActionRow;

use std::fmt::Write;

struct LeaderboardListing<'a> {
    pages: i32,
    curr_page: i32,

    scores: &'a Vec<OsuScore>,
    beatmap: &'a OsuBeatmap,

    embed: Embed,
}

impl<'a> LeaderboardListing<'a> {
    fn new(s: &'a Vec<OsuScore>, b: &'a OsuBeatmap) -> LeaderboardListing {
        let pages: i32 = (s.len() as f32 / 10.0 ).ceil() as i32;

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
                _ => s.pp.unwrap_or(0.0).to_string() + "pp",
            };

            let _ = writeln!(st, "{} • {:.2}% • {:.2} • {}",
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

pub async fn run(ctx: &FumoContext, command: InteractionCommand) {
    command.defer(&ctx).await.unwrap();
    let mut msg = MessageBuilder::new();

    let mut bid: i32 = -1;
    
    // If we got direct link
    if let Some(link) = command.get_option_string("link") {
        if let Some(v) = parse_link(link) {
            bid = v;
        } else {
            msg = msg.content("Invalid link format!");
            command.update(&ctx, &msg).await.unwrap();
            return;
        }
    }

    // If we got app interaction
    
    // If we got basic interaction without direct link
    //

    // If bid is still -1 after all
    if bid == -1 {
        msg = msg.content("Couldn't find any score/beatmap!");
        command.update(&ctx, &msg).await.unwrap();
        return;
    }


    let clb = match ctx.osu_api.get_countryleaderboard(bid).await {
        Some(lb) => lb,
        None => {
            msg = msg.content("Issues with leaderboard api. blame seneal");
            command.update(&ctx, &msg).await.unwrap();
            return;
        }
    };

    let b = match ctx.osu_api.get_beatmap(bid).await {
        Some(b) => b,
        None => {
            msg = msg.content("Issues with osu!api. blame peppy");
            command.update(&ctx, &msg).await.unwrap();
            return;
        }
    };

    let lb = LeaderboardListing::new(&clb.scores, &b);

    msg = msg.embed(lb.embed.clone());
    msg = msg.components(LeaderboardListing::components());
    command.update(&ctx, &msg).await.unwrap();
}
