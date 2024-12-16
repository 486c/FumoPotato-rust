use crate::{fumo_context::FumoContext, utils::interaction::InteractionComponent, };

/// A trait for Listing (Pagination)
///
/// A helper macro to reduce boiler plate code
/// can be found in `fumo_macro` crate
pub trait ListingTrait {
    /// Handling associated interactions
    /// Example: embed buttons clicks
    async fn handle_interaction_component(
        &mut self,
        ctx: &FumoContext,
        component: &InteractionComponent,
    );

    /// Update message/embeds/attachments according to the new page
    fn update(&mut self);
}
