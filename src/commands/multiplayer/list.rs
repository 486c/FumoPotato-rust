use fumo_database::osu::OsuDbMatch;
use twilight_model::application::interaction::InteractionData;
use twilight_model::application::interaction::Interaction;
use std::time::Duration;
use fumo_macro::listing;
use fumo_twilight::message::MessageBuilder;
use osu_api::models::{OsuUserExtended, UserId};
use twilight_interactions::command::{CommandModel, CommandOption, CreateCommand, CreateOption};
use twilight_model::channel::message::MessageFlags;
use twilight_util::builder::embed::{EmbedBuilder, EmbedFooterBuilder};
use eyre::Result;
use std::fmt::Write;
use tokio_stream::StreamExt;

use crate::{components::listing::ListingTrait, fumo_context::FumoContext, utils::{interaction::{InteractionCommand, InteractionComponent}, static_components::pages_components}};

use super::ListKind;

#[listing]
pub struct MatchesListing {
    pub osu_matches: Vec<OsuDbMatch>,
    pub osu_user: OsuUserExtended,
}

impl ListingTrait for MatchesListing {
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
        let footer = EmbedFooterBuilder::new(format!(
            "Total: {} • Page {}/{}",
            self.osu_matches.len(),
            self.current_page,
            self.max_pages,
        ));

        let start_at = (self.current_page - 1) * self.entries_per_page;
        let matches_iter = self
            .osu_matches
            .iter()
            .skip(start_at)
            .take(self.entries_per_page);

        let mut description_str = String::new();
        for m in matches_iter {
            let _ = writeln!(
                description_str, 
                "- **[{}](https://osu.ppy.sh/community/matches/{})**", 
                m.name,
                m.id
            );
            let _ = writeln!(
                description_str,
                "<t:{}:R>",
                m.start_time.and_utc().timestamp()
            );
        }

        let embed = EmbedBuilder::new()
            .color(123432)
            .title(format!("Matches History for {}", &self.osu_user.username))
            .description(description_str)
            .footer(footer)
            .build();

        self.embed = Some(embed);
    }
}

/// List all matches which player participated in
#[derive(CommandModel, CreateCommand, Debug)]
#[command(name = "list")]
pub struct MultiplayerList {
    /// List all matches or only in tournament related
    kind: ListKind,

    /// osu! user id or username
    user: Option<String>
}

impl MultiplayerList {
    pub async fn run(
        &self,
        ctx: &FumoContext,
        cmd: InteractionCommand,
    ) -> Result<()> {
        
        let osu_user_id = match &self.user {
            Some(value) => {
                UserId::from(value.as_ref())
            },
            None => {
                let osu_user = osu_user!(ctx, cmd);

                if osu_user.is_none() {
                    let msg = MessageBuilder::new()
                        .flags(MessageFlags::EPHEMERAL)
                        .content("No linked account found!");
                    cmd.response(ctx, &msg).await?;
                    return Ok(());
                }


                let osu_user_db = osu_user.unwrap();
                UserId::Id(osu_user_db.osu_id)
            },
        };

        cmd.defer(ctx).await?;

        let osu_api_user = ctx
            .osu_api
            .get_user(
                osu_user_id,
                None, // TODO: support gamemodes
            )
            .await?;

        if osu_api_user.is_none() {
            let msg = MessageBuilder::new()
                .flags(MessageFlags::EPHEMERAL)
                .content("Are you restricted? Can't find user id on osu!");
            cmd.update(ctx, &msg).await?;
            return Ok(());
        }

        let osu_api_user = osu_api_user.unwrap();

        let matches = match self.kind {
            ListKind::All => {
                ctx.db.get_user_matches_all(osu_api_user.id).await?
            }
            ListKind::Tournament => {
                ctx.db.get_user_matches_tourney(osu_api_user.id).await?
            }
        };

        let matches_len = matches.len();

        let mut matches_list = MatchesListing::new(matches, osu_api_user)
            .calculate_pages(matches_len, 10);

        matches_list.update();

        let mut msg_builder = MessageBuilder::new()
            .embed(
                matches_list
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
            matches_list
                .handle_interaction_component(ctx, &component)
                .await;
            matches_list.update();

            msg_builder = msg_builder.embed(
                matches_list
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
}
