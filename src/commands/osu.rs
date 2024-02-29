use eyre::{Result, bail};
use twilight_model::{application::interaction::
    application_command::CommandOptionValue::{
        self,
        String as OptionString
}, channel::message::MessageFlags};


use crate::{
    fumo_context::FumoContext, 
    utils::{InteractionCommand, MessageBuilder}, osu_api::{models::UserId, error::OsuApiError}
};

pub async fn osu_unlink(
    ctx: &FumoContext, 
    command: &InteractionCommand,
) -> Result<()> {
    let osu_user = osu_user!(ctx, command);
    let mut msg = MessageBuilder::new()
        .flags(MessageFlags::EPHEMERAL);

    if osu_user.is_none() {
        msg = msg
            .content("No linked account found!");
        command.response(ctx, &msg).await?;
        return Ok(())
    }

    ctx.db.unlink_osu(discord_id!(command).get() as i64)
        .await?;

    msg = msg
        .content("Successfully unlinked account!");

    command.response(ctx, &msg).await?;

    Ok(())
}

pub async fn osu_link(
    ctx: &FumoContext, 
    command: &InteractionCommand,
    name: &str
) -> Result<()> {
    let osu_user = osu_user!(ctx, command);
    let mut msg = MessageBuilder::new()
        .flags(MessageFlags::EPHEMERAL);

    if osu_user.is_some() {
        msg = msg.content(
            r#"You already have linked account. Please use `/unlink` to unlink it."#
        );

        command.response(ctx, &msg).await?;
        return Ok(())
    }

    let user = ctx.osu_api.get_user(
        UserId::Username(name.to_owned()), 
        None
    ).await;

    if let Err(OsuApiError::NotFound{..}) = user {
        msg = msg
            .content("User not found!");
        command.response(ctx, &msg).await?;
        return Ok(())
    }

    let user = user?;

    ctx.db.link_osu(
        discord_id!(command).get() as i64,
        user.id
    ).await?;

    msg = msg
        .content("Successfully linked account!");

    command.response(ctx, &msg).await?;

    Ok(())
}

pub async fn run(
    ctx: &FumoContext, 
    command: InteractionCommand
) -> Result<()> {
    if let Some(option) = command.data.options.first() {
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
