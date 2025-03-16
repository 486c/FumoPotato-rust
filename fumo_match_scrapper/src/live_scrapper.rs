use std::{collections::HashMap, sync::Arc, time::{Duration, Instant}};

use fumo_database::Database;
use osu_api::{models::osu_matches::OsuMatchGet, OsuApi};
use tokio::sync::{mpsc::UnboundedSender, RwLock};
use tokio_util::sync::CancellationToken;

static CHECK_FOR_END_INTERVAL: u64 = 60;
static CHECK_INTERVAL_OVERALL: u64 = 30;

/*

Live scrapper consists of two threads
    1. Reguarly checks for a new matches
    2. Reguarly checks for matches to end
*/

async fn fetch_new_matches(
    last_id: &mut Option<i64>,
    osu_api: &OsuApi,
    db_sender: &UnboundedSender<Box<OsuMatchGet>>,
    end_queue: &RwLock<HashMap<i64, Instant>>,
    db: &Database,
) -> eyre::Result<()> {
    let mut buffer = Vec::with_capacity(100);

    let fetched_matches = osu_api.get_matches_batch(&None).await?;

    if fetched_matches.matches.is_empty() {
        println!("Fetched matches is empty for some reason");
        return Ok(())
    }

    let newest = fetched_matches.matches.first().ok_or_else(|| eyre::eyre!("Failed to get newest match id!"))?;

    // Collecting a newly appeared match_id's
    for fetched_match in &fetched_matches.matches {
        if let Some(last_id) = last_id {
            // Skipping already checked ones
            if fetched_match.id < *last_id {
                continue
            }
        }

        // Checking if match is really ended
        // if not sending it to the checking queue
        if fetched_match.end_time.is_none() {
            println!("[{}] Not ended yet, pushing to the queue", fetched_match.id);
            let mut lock = end_queue.write().await;

            if lock.contains_key(&fetched_match.id) {
                continue;
            }

            lock.insert(fetched_match.id, Instant::now());

            continue;
        }
        
        // Adding to the regular queue
        buffer.push(fetched_match.id);
    }

    // Double checking if match actually exists in db
    let matches = db.is_osu_match_exists_batch(&buffer).await?;

    // Start fetching matches
    // and splitting them into corresponding containers
    println!("Found new matches: {}", buffer.len());
    for db_match in matches.iter().filter(|x| !x.exists) {
        let match_id = db_match.id;
        let fetch_result = osu_api.get_match_all_events(match_id).await;

        match fetch_result {
            Ok(data) => {
                println!("Fetched {}", match_id);
                let boxed_data = Box::new(data);
                let _ = db_sender.send(boxed_data);
            },
            Err(e) => match e {
                osu_api::error::OsuApiError::NotFound { .. } => {
                    println!("[{}] Not found", match_id);
                    continue
                },
                _ => println!("[{}] Error during request: {e}", match_id),
            },
        }
    }
    
    *last_id = Some(newest.id);

    Ok(())
}

async fn new_matches_worker(
    cancel_token: CancellationToken,
    db_sender: UnboundedSender<Box<OsuMatchGet>>,
    osu_api: Arc<OsuApi>,
    end_queue: Arc<RwLock<HashMap<i64, Instant>>>,
    db: Arc<Database>,
) {
    println!("Running live worker");
    let mut last_id: Option<i64> = None;

    while !cancel_token.is_cancelled() {
        let res = fetch_new_matches(
            &mut last_id,
            &osu_api,
            &db_sender,
            &end_queue,
            &db
        ).await;

        match res {
            Ok(_) => {},
            Err(e) => {
                println!("Error during new matches checker: {e}");
            },
        }
        
        // tokio select
        tokio::time::sleep(Duration::from_secs(30)).await;
    }
}

async fn process_queue(
    buffer: &mut Vec<i64>,
    osu_api: &OsuApi,
    db_sender: &UnboundedSender<Box<OsuMatchGet>>,
    end_queue: &RwLock<HashMap<i64, Instant>>
) -> eyre::Result<()> {
    // Reusing same buffer
    buffer.clear();

    let lock = end_queue.read().await;

    println!("Rechecking for ended matches: current queue size: {}", lock.len());
    
    // Appending a ready to check match ids
    // in order to avoid locking for write
    for (match_id, last_checked) in lock.iter() {
        if last_checked.elapsed() < Duration::from_secs(CHECK_FOR_END_INTERVAL) {
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
                    println!("[queue_processor][{}] Match is not yet disbaned", match_id);
                    let mut lock = end_queue.write().await;

                    lock.entry(*match_id).and_modify(|x| *x = Instant::now());
                    continue;
                }


                println!("[queue_processor] Fetched {}", match_id);
                let boxed_data = Box::new(data);
                let _ = db_sender.send(boxed_data);

                let mut lock = end_queue.write().await;
                lock.remove(match_id);
            },
            Err(e) => match e {
                osu_api::error::OsuApiError::NotFound { .. } => {
                    println!("[{}] Not found", match_id);
                    continue
                },
                _ => println!("[queue_processor][{}] Error during request: {e}", match_id),
            }
        }
    }

    Ok(())

}

async fn queue_worker(
    cancel_token: CancellationToken,
    db_sender: UnboundedSender<Box<OsuMatchGet>>,
    osu_api: Arc<OsuApi>,
    end_queue: Arc<RwLock<HashMap<i64, Instant>>>,
) {
    println!("Running queue worker");
    let mut buffer = Vec::with_capacity(10);

    while !cancel_token.is_cancelled() {
        let res = process_queue(
            &mut buffer,
            &osu_api,
            &db_sender,
            &end_queue
        ).await;

        match res {
            Ok(_) => {},
            Err(e) => {
                println!("Error during queue checker: {e}");
            },
        }

        tokio::time::sleep(Duration::from_secs(CHECK_INTERVAL_OVERALL)).await;
    }
}

pub async fn run(
    osu_api: Arc<OsuApi>,
    cancel_token: CancellationToken,
    db_sender: UnboundedSender<Box<OsuMatchGet>>,
    db: Arc<Database>,
) {
    let end_queue: RwLock<HashMap<i64, Instant>> = RwLock::new(HashMap::new());
    let arc_end_queue = Arc::new(end_queue);

    tokio::spawn(new_matches_worker(
        cancel_token.clone(),
        db_sender.clone(),
        osu_api.clone(),
        arc_end_queue.clone(),
        db.clone()
    ));

    tokio::spawn(queue_worker(
        cancel_token.clone(),
        db_sender.clone(),
        osu_api.clone(),
        arc_end_queue.clone()
    ));
}
