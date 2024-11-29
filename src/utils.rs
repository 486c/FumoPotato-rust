use std::{future::IntoFuture, slice};

use fumo_twilight::message::MessageBuilder;
use once_cell::sync::OnceCell;

use twilight_http::response::{marker::EmptyBody, ResponseFuture};

use twilight_model::{
    channel::{
        message::component::{ActionRow, Button, ButtonStyle},
        Channel,
    },
    guild::PartialMember,
    http::interaction::{InteractionResponse, InteractionResponseType},
    id::marker::UserMarker,
    user::User,
};

use twilight_model::channel::message::{component::Component, Message};

use twilight_model::id::{
    marker::{ChannelMarker, GuildMarker, InteractionMarker},
    Id,
};

use twilight_model::application::interaction::{
    application_command::{CommandData, CommandDataOption, CommandOptionValue},
    message_component::MessageComponentInteractionData,
    InteractionType,
};
use twilight_util::builder::InteractionResponseDataBuilder;

use crate::fumo_context::FumoContext;

#[derive(Debug)]
pub struct InteractionComponent {
    pub channel: Option<Channel>,
    pub data: Option<MessageComponentInteractionData>,
    pub kind: InteractionType,
    pub guild_id: Option<Id<GuildMarker>>,
    pub id: Id<InteractionMarker>,
    pub token: String,
}

impl InteractionComponent {
    pub fn defer(&self, ctx: &FumoContext) -> ResponseFuture<EmptyBody> {
        let response = InteractionResponse {
            kind: InteractionResponseType::DeferredUpdateMessage,
            data: None,
        };

        ctx.interaction()
            .create_response(self.id, &self.token, &response)
            .into_future()
    }
}

#[derive(Debug)]
pub struct InteractionCommand {
    pub channel_id: Id<ChannelMarker>,
    pub data: CommandData,
    pub kind: InteractionType,
    pub guild_id: Option<Id<GuildMarker>>,
    pub id: Id<InteractionMarker>,
    pub token: String,
    pub member: Option<PartialMember>,
    pub user: Option<User>,
}

impl InteractionCommand {
    pub fn defer(&self, ctx: &FumoContext) -> ResponseFuture<EmptyBody> {
        let response = InteractionResponse {
            kind: InteractionResponseType::DeferredChannelMessageWithSource,
            data: None,
        };

        ctx.interaction()
            .create_response(self.id, &self.token, &response)
            .into_future()
    }

    pub fn update<'a>(
        &self,
        ctx: &'a FumoContext,
        builder: &'a MessageBuilder,
    ) -> ResponseFuture<Message> {
        let client = ctx.interaction();
        let mut req = client.update_response(&self.token);

        if let Some(ref content) = builder.content {
            req = req
                .content(Some(content.as_ref()))
                .expect("invalid content!");
        }

        if let Some(ref embed) = builder.embed {
            req = req
                .embeds(Some(slice::from_ref(embed)))
                .expect("invalid embed!");
        }

        if let Some(ref components) = builder.components {
            req = req
                .components(Some(components.as_slice()))
                .expect("invalid components!");
        }

        if let Some(ref attachments) = builder.attachments {
            req = req
                .attachments(attachments.as_slice())
                .expect("invalid embed!");
        }

        req.into_future()
    }

    pub fn response<'a>(
        &self,
        ctx: &'a FumoContext,
        builder: &'a MessageBuilder,
    ) -> ResponseFuture<EmptyBody> {
        let mut data = InteractionResponseDataBuilder::new();

        // TODO Remove cloning

        if let Some(ref content) = builder.content {
            data = data.content(content)
        }

        if let Some(ref embed) = builder.embed {
            data = data.embeds([embed.clone()])
        }

        if let Some(ref components) = builder.components {
            data = data.components(components.clone())
        }

        if let Some(ref attachments) = builder.attachments {
            data = data.attachments(attachments.clone())
        }

        if let Some(ref flags) = builder.flags {
            data = data.flags(*flags);
        }

        let response = InteractionResponse {
            kind: InteractionResponseType::ChannelMessageWithSource,
            data: Some(data.build()),
        };

        ctx.interaction()
            .create_response(self.id, &self.token, &response)
            .into_future()
    }

    pub fn user_id(&self) -> Option<Id<UserMarker>> {
        if let Some(member) = &self.member {
            if let Some(user) = &member.user {
                return Some(user.id);
            }
        }

        if let Some(user) = &self.user {
            return Some(user.id);
        }

        None
    }

    #[inline]
    pub fn get_option(&self, name: &str) -> Option<&CommandDataOption> {
        self.data.options.iter().find(|x| x.name == name)
    }

    #[inline]
    pub fn get_option_string(&self, name: &str) -> Option<&str> {
        if let Some(option) = self.get_option(name) {
            if let CommandOptionValue::String(v) = &option.value {
                return Some(v.as_str());
            }
        };
        None
    }
}

pub struct Regex {
    regex: &'static str,
    cell: OnceCell<regex::Regex>,
}

impl Regex {
    const fn new(regex: &'static str) -> Self {
        Self {
            regex,
            cell: OnceCell::new(),
        }
    }

    pub fn get(&self) -> &regex::Regex {
        self.cell
            .get_or_init(|| regex::Regex::new(self.regex).unwrap())
    }
}

macro_rules! define_regex {
    ($($name:ident: $pat:literal;)*) => {
        $( pub static $name: Regex = Regex::new($pat); )*
    };
}

define_regex! {
    OSU_MAP_ID_NEW: r"https://osu.ppy.sh/beatmapsets/(\d+)(?:(?:#(?:osu|mania|taiko|fruits)|<#\d+>)/(\d+))?";
    OSU_MAP_ID_OLD: r"https://osu.ppy.sh/b(?:eatmaps)?/(\d+)";
}

#[macro_export]
macro_rules! random_string {
    ($count:expr) => {
        Alphanumeric.sample_string(&mut rand::thread_rng(), $count)
    };
}

#[inline]
pub fn hit_windows_circle_std(od: f64) -> (f64, f64, f64) {
    (80.0 - 6.0 * od, 140.0 - 8.0 * od, 200.0 - 10.0 * od)
}

#[inline]
pub fn ar_to_ms(ar: f64) -> f64 {
    if ar > 5.0 {
        1200.0 - 750.0 * (ar - 5.0) / 5.0
    } else if ar < 5.0 {
        1200.0 + 600.0 * (5.0 - ar) / 5.0
    } else {
        1200.0
    }
}

#[inline]
pub fn ms_to_ar(ms: f64) -> f64 {
    if ms < 1200.0 {
        ((ms * 5.0 - 1200.0 * 5.0) / (450.0 - 1200.0)) + 5.0
    } else if ms > 1200.0 {
        5.0 - ((1200.0 * 5.0 - ms * 5.0) / (1200.0 - 1800.0))
    } else {
        1200.0
    }
}

pub fn pages_components() -> Vec<Component> {
    let mut vec = Vec::with_capacity(2);

    let button = Component::Button(Button {
        custom_id: Some("B1".to_owned()),
        disabled: false,
        label: Some("Prev".to_owned()),
        style: ButtonStyle::Primary,
        url: None,
        emoji: None,
    });
    vec.push(button);

    let button = Component::Button(Button {
        custom_id: Some("B2".to_owned()),
        disabled: false,
        label: Some("Next".to_owned()),
        style: ButtonStyle::Primary,
        url: None,
        emoji: None,
    });
    vec.push(button);

    let component = Component::ActionRow(ActionRow { components: vec });

    vec![component]
}
