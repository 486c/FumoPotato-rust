use prometheus::{ IntCounterVec, Registry };

pub struct BotStats {
    pub registry: Registry,
    pub osu_api: IntCounterVec,
}

impl BotStats {
    pub fn new(osu_metrics: IntCounterVec) -> Self {
        let registry = Registry::new_custom(Some(String::from("fumo_potato")), None).unwrap();

        registry.register(Box::new(osu_metrics.clone())).unwrap();

        Self {
            registry,
            osu_api: osu_metrics,
        }
    }
}
