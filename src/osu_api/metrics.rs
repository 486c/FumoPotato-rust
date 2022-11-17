use prometheus::{IntCounter, IntCounterVec, Opts};

pub struct Metrics {
    pub counters: IntCounterVec,

    pub beatmap: IntCounter,
    pub country_leaderboard: IntCounter,
}

impl Metrics {
    pub fn new() -> Self {
        let opts = Opts::new("osu_requests", "osu!api requests");
        let counters = IntCounterVec::new(opts, &["type"]).unwrap();


        Self {
            beatmap: counters.with_label_values(&["beatmap"]),
            country_leaderboard: counters.with_label_values(&["country"]),

            counters,
        }

    }
}
