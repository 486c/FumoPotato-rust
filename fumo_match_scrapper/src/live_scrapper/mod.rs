use std::{collections::HashMap, sync::Arc, time::Instant};

use fumo_database::Database;
use osu_api::{models::osu_matches::OsuMatchGet, OsuApi};
use tokio::sync::{mpsc::UnboundedSender, RwLock};
use tokio_util::sync::CancellationToken;

static CHECK_FOR_END_INTERVAL: u64 = 60;
static CHECK_INTERVAL_OVERALL: u64 = 30;

mod matches_worker;
mod queue_worker;

// Live scrapper consists of two workers
//  1. Reguarly checks for a new matches
//  2. Reguarly checks for matches to end

pub async fn run(
    osu_api: Arc<OsuApi>,
    cancel_token: CancellationToken,
    db_sender: UnboundedSender<Box<OsuMatchGet>>,
    db: Arc<Database>,
) {
    let end_queue: RwLock<HashMap<i64, Instant>> = RwLock::new(HashMap::new());
    let arc_end_queue = Arc::new(end_queue);

    tokio::spawn(matches_worker::run(
        cancel_token.clone(),
        db_sender.clone(),
        osu_api.clone(),
        arc_end_queue.clone(),
        db.clone(),
    ));

    tokio::spawn(queue_worker::run(
        cancel_token.clone(),
        db_sender.clone(),
        osu_api.clone(),
        arc_end_queue.clone(),
    ));
}
