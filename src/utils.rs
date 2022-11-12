use twilight_model::http::interaction::{InteractionResponseData, InteractionResponse};
use twilight_model::http::interaction::InteractionResponseType;
use twilight_http::response::{marker::EmptyBody, ResponseFuture};
use twilight_model::channel::message::Message;

use twilight_model::id::{
    Id, 
    marker::{ ChannelMarker, GuildMarker, InteractionMarker }
};

use twilight_model::application::interaction::{ 
    Interaction, InteractionType, InteractionData,
    application_command::CommandData
};

use crate::fumo_context::FumoContext;

#[derive(Debug)]
pub struct MessageBuilder {
    content: Option<String>,
}

impl MessageBuilder {
    pub fn new() -> Self {
        MessageBuilder {
            content: None,
        }
    }

    pub fn content(mut self, s: impl Into<String>) -> Self {
        self.content = Some(s.into());
        self
    }
}

#[derive(Debug)]
pub struct InteractionCommand {
    pub channel_id: Id<ChannelMarker>,
    pub data: Box<CommandData>,
    pub kind: InteractionType,
    pub guild_id: Option<Id<GuildMarker>>,
    pub id: Id<InteractionMarker>,
    pub token: String
}

impl InteractionCommand {
    /*
    pub fn defer_update(
        &self, 
        ctx: &FumoContext, 
        builder: MessageBuilder
    ) -> ResponseFuture<EmptyBody> {
        let data = InteractionResponseData {
            content: builder.content,
            ..Default::default()
        };

        let response = InteractionResponse {
            kind: InteractionResponseType::DeferredUpdateMessage,
            data: Some(data),
        };

        ctx.interaction().
            create_response(
                self.id,
                &self.token,
                &response
            )
            .exec()
    }
    */

    pub fn defer(&self, ctx: &FumoContext) -> ResponseFuture<EmptyBody> {
        let response = InteractionResponse {
            kind: InteractionResponseType::DeferredChannelMessageWithSource,
            data: None,
        };

        ctx.interaction().
            create_response(
                self.id,
                &self.token,
                &response
            )
            .exec()
    }

    pub fn update<'a>(
        &self, 
        ctx: &'a FumoContext,
        builder: &'a MessageBuilder,
    ) -> ResponseFuture<Message> {
        let client = ctx.interaction();
        let mut req = client.update_response(&self.token);

        if let Some(ref content) = builder.content {
            req = req.content(Some(content.as_ref()))
                    .expect("invalid content!");
        }

        req.exec()
    }
}
