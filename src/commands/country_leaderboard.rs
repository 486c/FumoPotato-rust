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

struct LeaderboardListing<'a> {
    pages: i32,
    curr_page: i32,

    scores: &'a Vec<OsuScore>,
    beatmap: &'a OsuBeatmap,
}

impl<'a> LeaderboardListing<'a> {
    fn init(s: &'a Vec<OsuScore>, b: &'a OsuBeatmap) -> LeaderboardListing {
        let pages: i32 = if (s.len() as f32) / 10.0 < 1.0 {
            1
        } else {
            (s.len() as i32) / 10
        };

        LeaderboardListing {
            pages,
            curr_page: 1,
            scores: s,
            beatmap: b,
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

    fn create_embed<'b>(&self, embed: &'b mut CreateEmbed) -> &'b mut CreateEmbed {
        embed.author(|a| {
            a.name(format!(
                "Total scores: 50 [{}/{}]",
                self.curr_page, self.pages
            ))
        });

        embed.thumbnail(format!(
            "https://assets.ppy.sh/beatmaps/{}/covers/list.jpg",
            self.beatmap.beatmapset_id
        ));

        embed.color(Colour::new(865846));

        embed.footer(|f| {
            f.text(self.beatmap.metadata())
        });

        let mut st = String::new();

        let mut count = 10;
        let mut index = 1 + (self.curr_page - 1) * 10;
        for s in self
            .scores
            .iter()
            .skip(((self.curr_page - 1) * 10) as usize)
        {
            if count == 0 {
                break;
            }

            let pp = s.pp.unwrap_or(0.0);

            st.push_str(
                format!(
                    "{}. [{}](https://osu.ppy.sh/u/{}) 
                    {} • {:.1} • {:.2}pp 
                    {} 
                    <t:{}:R>\n",
                    index,
                    s.user.username,
                    s.user.id,
                    s.rank,
                    s.accuracy * 100.0,
                    pp,
                    s.score.to_formatted_string(&Locale::en),
                    s.created_at.timestamp()
                )
                .as_str(),
            );

            index += 1;
            count -= 1;
        }

        embed.description(st)
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

pub async fn run(ctx: &Context, command: &ApplicationCommandInteraction) {
    /*
    let mut bid: i32 = -1;

    command
        .create_interaction_response(&ctx.http, |response| {
            response.kind(InteractionResponseType::DeferredChannelMessageWithSource)
        })
        .await
        .unwrap();

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
    } else {
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
            .edit_original_interaction_response(&ctx.http, |m| m.content("Can't find any beatmap!"))
            .await
            .unwrap();
    }

    let api = OSU_API.get().unwrap();

    let cb = match api.get_countryleaderboard(bid).await {
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

    let b = match api.get_beatmap(bid).await {
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
            m.embed(|e| {
                lb.create_embed(e);
                e
            })
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

        command
            .edit_original_interaction_response(&ctx.http, |m| {
                m.content("placeholder").embed(|e| {
                    lb.create_embed(e);
                    e
                })
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
    */
}
