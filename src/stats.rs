use prometheus::{IntCounterVec, Opts, Registry};

pub struct BotStats {
    /// Command usage counters
    pub command_counters: IntCounterVec,
}

impl BotStats {
    pub fn new() -> Self {
        let opts = Opts::new("fumo_bot_commands", "specific commands usage");
        let command_counters = IntCounterVec::new(opts, &["name"]).unwrap();

        Self {
            command_counters,
        }
    }
}

pub struct BotMetrics {
    pub registry: Registry,
    pub osu_api: IntCounterVec,
    pub bot: IntCounterVec,
}

impl BotMetrics {
    pub fn new(
        osu_metrics: IntCounterVec,
        bot_metrics: IntCounterVec,
    ) -> Self {
        let registry =
            Registry::new_custom(Some(String::from("fumo_potato")), None)
                .unwrap();

        registry.register(Box::new(osu_metrics.clone())).unwrap();
        registry.register(Box::new(bot_metrics.clone())).unwrap();

        Self {
            registry,
            osu_api: osu_metrics,
            bot: bot_metrics,
        }
    }
}
