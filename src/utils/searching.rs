use twilight_model::channel::Message;

use super::{OSU_MAP_ID_NEW, OSU_MAP_ID_OLD};

pub fn find_beatmap_link(msg: &Message) -> Option<&String> {
    match msg.author.id.get() {
        // owo bot
        289066747443675143 => msg.embeds.first()?.author.as_ref()?.url.as_ref(),
        // bath bot & mikaizuku
        297073686916366336 | 839937716921565252 => {
            msg.embeds.first()?.url.as_ref()
        }
        _ => None,
    }
}

pub fn parse_beatmap_link(str: &str) -> Option<i32> {
    if !str.contains("https://osu.ppy.sh") {
        return None;
    }

    let m = if let Some(o) = OSU_MAP_ID_OLD.get().captures(str) {
        o.get(1)
    } else {
        OSU_MAP_ID_NEW.get().captures(str).and_then(|o| o.get(2))
    };

    m.and_then(|o| o.as_str().parse().ok())
}
