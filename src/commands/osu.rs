use eyre::{Result, bail};
use twilight_model::application::interaction::
    application_command::CommandOptionValue::{
        self,
        String as OptionString
};


use crate::{
    fumo_context::FumoContext, 
    utils::{InteractionCommand, MessageBuilder}, osu_api::{models::UserId, error::OsuApiError}
};

pub async fn osu_unlink(
    ctx: &FumoContext, 
    command: &InteractionCommand,
) -> Result<()> {
    command.defer(ctx).await?;
    let osu_user = osu_user!(ctx, command);

    if osu_user.is_none() {
        let msg = MessageBuilder::new()
            .content("No linked account found!");
        command.update(ctx, &msg).await?;
        return Ok(())
    }

    ctx.db.unlink_osu(discord_id!(command).get() as i64)
        .await?;

    let msg = MessageBuilder::new()
        .content("Successfully unlinked account!");

    command.update(ctx, &msg).await?;

    Ok(())
}

pub async fn osu_link(
    ctx: &FumoContext, 
    command: &InteractionCommand,
    name: &str
) -> Result<()> {
    command.defer(ctx).await?;

    let osu_user = osu_user!(ctx, command);

    if osu_user.is_some() {
        let msg = MessageBuilder::new()
            .content(
                r#"You already have linked account. Please use `/unlink` to unlink it."#);
        command.update(ctx, &msg).await?;
        return Ok(())
    }

    let user = ctx.osu_api.get_user(
        UserId::Username(name.to_owned()), 
        None
    ).await;

    if let Err(OsuApiError::NotFound{..}) = user {
        let msg = MessageBuilder::new()
            .content("User not found!");
        command.update(ctx, &msg).await?;
        return Ok(())
    }

    let user = user?;

    ctx.db.link_osu(
        discord_id!(command).get() as i64,
        user.id
    ).await?;

    let msg = MessageBuilder::new()
        .content("Successfully linked account!");

    command.update(ctx, &msg).await?;

    Ok(())
}

pub async fn run(
    ctx: &FumoContext, 
    command: InteractionCommand
) -> Result<()> {
    if let Some(option) = command.data.options.get(0) {
        if let CommandOptionValue::SubCommand(args) = &option.value {
            match option.name.as_ref() {
                "link" => {
                    if let OptionString(name) = &args[0].value {
                        osu_link(ctx, &command, name).await
                    } else { bail!("No required option provided!");}
                },
                "unlink" => osu_unlink(ctx, &command).await,
                _ => todo!()
            }
        } else { bail!("No subcommand found") }
    } else { bail!("Required option is not found") }
}
