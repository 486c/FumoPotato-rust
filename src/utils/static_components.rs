// For static components such as buttons/embeds and other stuff
// that's gonna be reused a lot

use twilight_model::channel::message::{
    component::{ActionRow, Button, ButtonStyle},
    Component,
};

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
