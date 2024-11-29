use twilight_model::{
    channel::message::{Component, Embed, MessageFlags},
    http::attachment::Attachment,
};

#[derive(Debug, Default)]
pub struct MessageBuilder {
    pub content: Option<String>,
    pub embed: Option<Embed>,
    pub components: Option<Vec<Component>>,
    pub attachments: Option<Vec<Attachment>>,
    pub flags: Option<MessageFlags>,
}

impl MessageBuilder {
    pub fn new() -> Self {
        MessageBuilder {
            ..Default::default()
        }
    }

    pub fn flags(mut self, flags: impl Into<MessageFlags>) -> Self {
        self.flags = Some(flags.into());
        self
    }

    // TODO uncomment if ever gonna be used
    // pub fn attachments(
    // mut self,
    // attachments: impl Into<Vec<Attachment>>
    // ) -> Self {
    // self.attachments = Some(attachments.into());
    // self
    // }

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

    pub fn clear_components(&mut self) {
        if let Some(components) = &mut self.components {
            components.clear();
        }
    }
}
