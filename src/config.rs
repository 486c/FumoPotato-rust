use once_cell::sync::OnceCell;
use std::env;

static CONFIG: OnceCell<BotConfig> = OnceCell::new();

#[derive(Debug)]
pub struct BotConfig {
    pub fallback_api: String,
}

impl BotConfig {
    pub fn init() {
        let cfg = BotConfig {
            fallback_api: env::var("FALLBACK_API").expect("FALLBACK_API env variable is not found"),
        };

        CONFIG.set(cfg).unwrap();
    }

    pub fn get() -> &'static BotConfig {
        CONFIG.get().unwrap()
    }

    pub fn get_res() -> Option<&'static BotConfig> {
        CONFIG.get()
    }
}
