use prometheus::{IntCounterVec, Opts, Registry};

pub struct BotStats {
    /// Command usage counters
    pub cmd: IntCounterVec,
    pub cache: IntCounterVec,
}

impl BotStats {
    pub fn new() -> Self {
        let opts = Opts::new("fumo_bot_commands", "specific commands usage");
        let command_counters = IntCounterVec::new(opts, &["name"]).unwrap();


        let opts = Opts::new("fumo_bot_cache", "caches miss/hits/force_updates");
        let cache_counters = IntCounterVec::new(opts, &["kind"]).unwrap();

        Self {
            cmd: command_counters,
            cache: cache_counters,
        }
    }
}

pub struct BotMetrics {
    pub registry: Registry,
    pub osu_api: IntCounterVec,
    pub bot: BotStats,
}

impl BotMetrics {
    pub fn new(
        osu_metrics: IntCounterVec,
        bot_metrics: BotStats,
    ) -> Self {
        let registry =
            Registry::new_custom(Some(String::from("fumo_potato")), None)
                .unwrap();

        registry.register(Box::new(osu_metrics.clone())).unwrap();
        registry.register(Box::new(bot_metrics.cmd.clone())).unwrap();
        registry.register(Box::new(bot_metrics.cache.clone())).unwrap();

        Self {
            registry,
            osu_api: osu_metrics,
            bot: bot_metrics,
        }
    }
}
