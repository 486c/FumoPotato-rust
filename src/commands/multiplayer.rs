use crate::utils::InteractionComponent;
use eyre::Result;
use fumo_database::osu::OsuDbMatch;
use fumo_macro::listing;
use fumo_twilight::message::MessageBuilder;
use osu_api::models::{OsuUserExtended, UserId};
use std::{fmt::Write, time::Duration};
use tokio_stream::StreamExt;
use twilight_interactions::command::{
    CommandModel, CommandOption, CreateCommand, CreateOption,
};
use twilight_model::{
    application::interaction::{Interaction, InteractionData},
    channel::message::MessageFlags,
};
use twilight_util::builder::embed::{EmbedBuilder, EmbedFooterBuilder};

use crate::{
    components::listing::ListingTrait,
    fumo_context::FumoContext,
    utils::{pages_components, InteractionCommand},
};

/// All osu! multiplayer related commands
#[derive(CommandModel, CreateCommand, Debug)]
#[command(name = "multiplayer")]
pub enum MultiplayerCommands {
    #[command(name = "list")]
    List(MultiplayerList),
}

impl MultiplayerCommands {
    pub async fn handle(
        ctx: &FumoContext,
        cmd: InteractionCommand,
    ) -> Result<()> {
        let command = Self::from_interaction(cmd.data.clone().into())?;

        match command {
            MultiplayerCommands::List(command) => command.run(ctx, cmd).await,
        }
    }
}

#[derive(Debug, CommandOption, CreateOption)]
pub enum ListKind {
    #[option(name = "All", value = "all")]
    All = 0,
    #[option(name = "Tournament", value = "tournament")]
    Tournament = 1,
}

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
            .skip(start_at as usize)
            .take(self.entries_per_page);

        let mut description_str = String::new();
        for m in matches_iter {
            let _ = writeln!(description_str, "- **{}**", m.name);
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
    /// List all matches or only tournament related
    kind: ListKind,
}

impl MultiplayerList {
    pub async fn run(
        &self,
        ctx: &FumoContext,
        cmd: InteractionCommand,
    ) -> Result<()> {
        let osu_user = osu_user!(ctx, cmd);

        if osu_user.is_none() {
            let msg = MessageBuilder::new()
                .flags(MessageFlags::EPHEMERAL)
                .content("No linked account found!");
            cmd.response(ctx, &msg).await?;
            return Ok(());
        }

        cmd.defer(ctx).await?;

        let osu_user_db = osu_user.unwrap();

        let osu_user = ctx
            .osu_api
            .get_user(
                UserId::Id(osu_user_db.osu_id),
                None, // TODO: support gamemodes
            )
            .await?;

        if osu_user.is_none() {
            let msg = MessageBuilder::new()
                .flags(MessageFlags::EPHEMERAL)
                .content("Are you restricted? Can't find user id on osu!");
            cmd.response(ctx, &msg).await?;
        }

        let osu_user = osu_user.unwrap();

        let matches = match self.kind {
            ListKind::All => {
                ctx.db.get_user_matches_all(osu_user_db.osu_id).await?
            }
            ListKind::Tournament => {
                ctx.db.get_user_matches_tourney(osu_user_db.osu_id).await?
            }
        };

        let matches_len = matches.len();

        let mut matches_list = MatchesListing::new(matches, osu_user)
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
