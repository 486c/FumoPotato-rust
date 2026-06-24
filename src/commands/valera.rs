use std::path::PathBuf;

use fumo_twilight::message::MessageBuilder;
use rand::Rng;
use rand::distributions::{Alphanumeric, DistString};

use twilight_interactions::command::{CommandModel, CreateCommand};
use twilight_model::http::attachment::Attachment;
use twilight_util::builder::embed::{EmbedBuilder, ImageSource};

use crate::{fumo_context::FumoContext, random_string, utils::interaction::InteractionCommand};
use eyre::Result;

const IMAGE_MAX: usize = 267;

/// Random quote of valera456
#[derive(CommandModel, CreateCommand, Debug)]
#[command(name = "valera456")]
pub struct ValeraCommand {}

impl ValeraCommand {
    pub async fn handle(
        ctx: &FumoContext,
        cmd: InteractionCommand,
    ) -> Result<()> {
        let _command = Self::from_interaction(cmd.data.clone().into())?;

        let random: usize = {
            let mut rng = rand::thread_rng();
            rng.gen_range(1..=IMAGE_MAX)
        };
        
        let path = PathBuf::from(
            format!("./valera_images/{}.jpg", random)
        );

        let image = tokio::fs::read(path).await?;
        
        let filename = format!("{}.jpg", random_string!(16));
        let attach = [Attachment::from_bytes(filename, image, 1337)];
        let source = ImageSource::attachment(&attach[0].filename)?;

        let embed = EmbedBuilder::new()
            .color(0x97158a)
            .image(source)
            .build();

        let msg = MessageBuilder::new()
            .embed(embed)
            .attachments(&attach);

        cmd.response(ctx, &msg).await?;

        Ok(())
    }
}
