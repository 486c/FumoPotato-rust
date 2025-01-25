use prometheus::{IntCounterVec, Opts};

#[derive(Debug)]
pub struct Metrics {
    pub counters: IntCounterVec,
}

impl Metrics {
    pub fn new() -> Self {
        let opts = Opts::new("osu_requests", "osu!api requests");
        let counters = IntCounterVec::new(opts, &["type"]).unwrap();

        Self {
            counters,
        }
    }
}

impl Default for Metrics {
    fn default() -> Self {
        Metrics::new()
    }
}
