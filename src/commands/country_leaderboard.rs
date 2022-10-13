use serenity::builder::{CreateComponents, CreateEmbed};
use serenity::futures::StreamExt;
use serenity::model::application::component::ButtonStyle;
use serenity::model::application::interaction::InteractionResponseType;
use serenity::model::channel::Message;
use serenity::model::prelude::interaction::application_command::ApplicationCommandInteraction;
use serenity::prelude::Context;
use serenity::utils::Colour;
use std::time::Duration;

use num_format::{Locale, ToFormattedString};

use crate::osu_api::Body;
use crate::osu_api::{OsuBeatmap, OsuScore};
use crate::OSU_API;
use crate::fumo_context::FumoContext;

struct LeaderboardListing<'a> {
    pages: i32,
    curr_page: i32,

    scores: &'a Vec<OsuScore>,
    beatmap: &'a OsuBeatmap,

    embed: CreateEmbed,
}

impl<'a> LeaderboardListing<'a> {
    fn init(s: &'a Vec<OsuScore>, b: &'a OsuBeatmap) -> LeaderboardListing {
        let pages: i32 = if (s.len() as f32) / 10.0 < 1.0 {
            1
        } else {
            (s.len() as i32) / 10
        };

        let mut embed = CreateEmbed::default();
        embed.author(|a| {
            a.name(b.metadata());
            a.url(format!("https://osu.ppy.sh/b/{}", b.id))
        });
        embed.thumbnail(format!(
            "https://assets.ppy.sh/beatmaps/{}/covers/list.jpg",
            b.beatmapset_id
        ));
        embed.color(Colour::new(865846));

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

    fn create_components<'b>(&self, c: &'b mut CreateComponents) -> &'b mut CreateComponents {
        c.create_action_row(|row| {
            row.create_button(|b| {
                b.style(ButtonStyle::Primary);
                b.label("Previous");
                b.custom_id("B1")
            });
            row.create_button(|b| {
                b.style(ButtonStyle::Primary);
                b.label("Next");
                b.custom_id("B2")
            })
        })
    }

    fn update_embed(&mut self) {
        self.embed.footer(|f| {
            f.text(format!("Page {}/{}", self.curr_page, self.pages))
        });

        let mut st = String::new();
        
        let start_index = (self.curr_page-1)*10;

        let mut count = 10;
        let mut index = 1 + start_index;
        for s in self.scores.iter().skip(start_index as usize)
        {
            if count == 0 {
                break;
            }

            let pp = s.pp.unwrap_or(0.0);

            let profile_text = format!("{}. [{}](https://osu.ppy.sh/u/{}) +**{}**\n",
                                       index, s.user.username, s.user.id,
                                       s.mods.to_string());
            let stats_text = format!("{} • {:.2}% • {:.2}pp • {}\n",
                                     s.rank.to_emoji(), s.accuracy * 100.0, pp,
                                     s.score.to_formatted_string(&Locale::en));
            let combo_text = format!("[{}x/{}x] [{}/{}/{}/{}]\n",
                s.max_combo, self.beatmap.max_combo,
                s.stats.count300, s.stats.count100, s.stats.count50,
                s.stats.countmiss
            );
            let timestamp_text = format!("<t:{}:R>\n", s.created_at.timestamp());

            st.push_str(
                format!("{}{}{}{}",
                        profile_text,
                        stats_text,
                        combo_text,
                        timestamp_text
                ).as_str()
            );

            index += 1;
            count -= 1;
        }

        self.embed.description(st);
    }
}

fn parse_bid(str: &str) -> Option<i32> {
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

fn find_link(msg: &Message) -> Option<&String> {
    // owo bot
    if msg.author.id == 289066747443675143 {
        return msg.embeds[0].author.as_ref()?.url.as_ref();
    }

    // bath bot
    if msg.author.id == 297073686916366336 {
        return msg.embeds.get(0)?.url.as_ref();
    }

    // mikaizuku bot
    if msg.author.id == 839937716921565252 {
        return msg.embeds.get(0)?.url.as_ref();
    }

    None
}

pub async fn run(
    ctx: &Context, 
    fumo_ctx: &FumoContext,
    command: &ApplicationCommandInteraction
) {
    let mut bid: i32 = -1;

    command
        .create_interaction_response(&ctx.http, |response| {
            response.kind(InteractionResponseType::DeferredChannelMessageWithSource)
        })
        .await
        .unwrap();

    if let Some(targed_id) = command.data.target_id {
        let msg = ctx.http.get_message(
            command.channel_id.0,
            targed_id.0,
        ).await.unwrap();

        if let Some(link) = find_link(&msg) {
            if let Some(id) = parse_bid(link) {
                bid = id;
            }
        }
    }

    if let Some(link_command) = command.data.options.get(0) {
        if let Some(link) = &link_command.value.as_ref().unwrap().as_str() {
            match parse_bid(link) {
                Some(id) => bid = id,
                None => {
                    command
                        .edit_original_interaction_response(&ctx.http, |m| {
                            m.content("Invalid url format!")
                        })
                        .await
                        .unwrap();
                    return;
                }
            }
        }
    } else if bid == -1 {
        // Iterating through message history
        let mut messages = command.channel_id.messages_iter(&ctx).boxed();
        while let Some(message_result) = messages.next().await {
            match message_result {
                Ok(msg) => {
                    if let Some(link) = find_link(&msg) {
                        if let Some(id) = parse_bid(link) {
                            bid = id;
                            break;
                        }
                    }
                }
                Err(_e) => continue,
            }
        }
    }

    if bid == -1 {
        command
            .edit_original_interaction_response(&ctx.http, |m| 
                m.content("Can't find any beatmap!"
            ))
            .await
            .unwrap();
    }

    let cb = match fumo_ctx.osu_api.get_countryleaderboard(bid).await {
        Some(id) => id,
        None => {
            command
                .edit_original_interaction_response(&ctx.http, |m| {
                    m.content("Error occured! **Please try again!**")
                })
                .await
                .unwrap();
            return;
        }
    };

    let b = match fumo_ctx.osu_api.get_beatmap(bid).await {
        Some(id) => id,
        None => {
            command
                .edit_original_interaction_response(&ctx.http, |m| {
                    m.content("Can't fetch beatmap from osu!api. **Please try again!**")
                })
                .await
                .unwrap();
            return;
        }
    };

    let mut lb = LeaderboardListing::init(&cb.scores, &b);

    // Initial message
    command
        .edit_original_interaction_response(&ctx.http, |m| {
            m.set_embed(lb.embed.clone())
            .components(|c| lb.create_components(c))
        })
        .await
        .unwrap();

    // Waiting for components interactions
    let msg = command.get_interaction_response(&ctx.http).await.unwrap();
    let mut interaction_stream = msg
        .await_component_interactions(&ctx)
        .timeout(Duration::from_secs(20))
        .build();

    // Waiting loop
    while let Some(interaction) = interaction_stream.next().await {
        match interaction.data.custom_id.as_str() {
            "B1" => lb.prev_page(),
            "B2" => lb.next_page(),
            _ => break,
        };

        lb.update_embed();

        command
            .edit_original_interaction_response(&ctx.http, |m| {
                m.set_embed(lb.embed.clone())
            })
            .await
            .unwrap();

        interaction.defer(&ctx.http).await.unwrap();
    }

    // Removing all components
    command
        .edit_original_interaction_response(&ctx.http, |m| m.components(|c| c))
        .await
        .unwrap();
}
