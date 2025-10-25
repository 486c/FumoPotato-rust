use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};

use osu_api::{models::osu_matches::OsuMatchGet, OsuApi};
use tokio::sync::{mpsc::UnboundedSender, RwLock};
use tokio_util::sync::CancellationToken;

use crate::live_scrapper::{CHECK_FOR_END_INTERVAL, CHECK_INTERVAL_OVERALL};

async fn process_queue(
    buffer: &mut Vec<i64>,
    osu_api: &OsuApi,
    db_sender: &UnboundedSender<Box<OsuMatchGet>>,
    end_queue: &RwLock<HashMap<i64, Instant>>,
) -> eyre::Result<()> {
    // Reusing same buffer
    buffer.clear();

    let lock = end_queue.read().await;

    println!(
        "Rechecking for ended matches: current queue size: {}",
        lock.len()
    );

    // Appending a ready to check match ids
    // in order to avoid locking for write
    for (match_id, last_checked) in lock.iter() {
        if last_checked.elapsed() < Duration::from_secs(CHECK_FOR_END_INTERVAL)
        {
            continue;
        }

        buffer.push(*match_id);
    }

    drop(lock);

    for match_id in buffer {
        let res = osu_api.get_match_all_events(*match_id).await;

        match res {
            Ok(data) => {
                if !data.is_match_disbanded() {
                    println!(
                        "[queue_processor][{}] Match is not yet disbaned",
                        match_id
                    );
                    let mut lock = end_queue.write().await;

                    lock.entry(*match_id).and_modify(|x| *x = Instant::now());
                    continue;
                }

                println!("[queue_processor] Fetched {}", match_id);
                let boxed_data = Box::new(data);
                let _ = db_sender.send(boxed_data);

                let mut lock = end_queue.write().await;
                lock.remove(match_id);
            }
            Err(e) => match e {
                osu_api::error::OsuApiError::NotFound { .. } => {
                    println!("[{}] Not found", match_id);
                    continue;
                }
                _ => println!(
                    "[queue_processor][{}] Error during request: {e}",
                    match_id
                ),
            },
        }
    }

    Ok(())
}

pub async fn run(
    cancel_token: CancellationToken,
    db_sender: UnboundedSender<Box<OsuMatchGet>>,
    osu_api: Arc<OsuApi>,
    end_queue: Arc<RwLock<HashMap<i64, Instant>>>,
) {
    println!("Running queue worker");
    let mut buffer = Vec::with_capacity(10);

    while !cancel_token.is_cancelled() {
        let res =
            process_queue(&mut buffer, &osu_api, &db_sender, &end_queue).await;

        match res {
            Ok(_) => {}
            Err(e) => {
                println!("Error during queue checker: {e}");
            }
        }

        tokio::time::sleep(Duration::from_secs(CHECK_INTERVAL_OVERALL)).await;
    }
}
