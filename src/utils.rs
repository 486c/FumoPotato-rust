use std::slice;

use twilight_model::http::interaction::InteractionResponse;
use twilight_model::http::interaction::InteractionResponseType;
use twilight_http::response::{marker::EmptyBody, ResponseFuture};
use twilight_model::channel::message::{ component::Component, Message };

use twilight_model::id::{
    Id, 
    marker::{ ChannelMarker, GuildMarker, InteractionMarker }
};

use twilight_model::application::interaction::{ 
    InteractionType,
    application_command::CommandData,
    message_component::MessageComponentInteractionData,
};
use twilight_model::application::interaction::application_command::{ CommandOptionValue, CommandDataOption };

use twilight_model::channel::message::embed::Embed;

use crate::fumo_context::FumoContext;

#[derive(Debug, Default)]
pub struct MessageBuilder {
    content: Option<String>,
    embed: Option<Embed>,
    pub components: Option<Vec<Component>>,
}

impl MessageBuilder {
    pub fn new() -> Self {
        MessageBuilder {
            ..Default::default()
        }
    }

    pub fn content(mut self, s: impl Into<String>) -> Self {
        self.content = Some(s.into());
        self
    }

    pub fn embed(mut self, e: impl Into<Embed>) -> Self {
        self.embed = Some(e.into());
        self
    }

    pub fn components(mut self, components: Vec<Component>) -> Self {
        self.components = Some(components);
        self
    }

}

#[derive(Debug)]
pub struct InteractionComponent {
    pub channel_id: Option<Id<ChannelMarker>>,
    pub data: Option<MessageComponentInteractionData>,
    pub kind: InteractionType,
    pub guild_id: Option<Id<GuildMarker>>,
    pub id: Id<InteractionMarker>,
    pub token: String
}

impl InteractionComponent {
    pub fn defer(&self, ctx: &FumoContext) -> ResponseFuture<EmptyBody> {
        let response = InteractionResponse {
            kind: InteractionResponseType::DeferredUpdateMessage,
            data: None,
        };

        ctx.interaction()
            .create_response(self.id, &self.token, &response)
            .exec()
    }
}

#[derive(Debug)]
pub struct InteractionCommand {
    pub channel_id: Id<ChannelMarker>,
    pub data: Box<CommandData>,
    pub kind: InteractionType,
    pub guild_id: Option<Id<GuildMarker>>,
    pub id: Id<InteractionMarker>,
    pub token: String,
}

impl InteractionCommand {
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

        if let Some(ref embed) = builder.embed {
            req = req.embeds(Some(slice::from_ref(embed)))
                    .expect("invalid embed!");
        }

        if let Some(ref components) = builder.components {
            req = req.components(Some(components.as_slice()))
                    .expect("invalid components!");
        }

        req.exec()
    }

    pub fn get_option(
        &self, 
        name: &str
    ) -> Option<&CommandDataOption> {
        self.data.options.iter().find(|x| x.name == name)
    }
    
    pub fn get_option_string(
        &self,
        name: &str
    ) -> Option<&str> {
        if let Some(option) = self.get_option(name) {
            if let CommandOptionValue::String(v) = &option.value {
                return Some(v.as_str())
            }
        };
        None
    }
}
