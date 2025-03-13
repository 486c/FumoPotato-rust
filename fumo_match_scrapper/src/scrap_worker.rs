use std::sync::Arc;

use fumo_database::Database;
use osu_api::{models::osu_matches::OsuMatchGet, OsuApi};
use tokio::sync::mpsc::UnboundedSender;
use tokio_util::sync::CancellationToken;

use crate::match_not_found::MatchNotFoundList;


pub async fn run(
    range: Vec<i64>,
    token: CancellationToken,
    db: Arc<Database>,
    osu_api: Arc<OsuApi>,
    sender: UnboundedSender<Box<OsuMatchGet>>,
    match_not_found_list: Arc<MatchNotFoundList>,
    batch_size: usize,
) {
    let mut to_process: Vec<i64> = Vec::new();

    for chunk in range.chunks(batch_size) {
        if token.is_cancelled() {
            break;
        }

        let matches_exists = db.is_osu_match_exists_batch(
            chunk
        )
        .await
        .unwrap();

        for match_record in matches_exists.iter().filter(|x| !x.exists) {
            let match_id = match_record.id;

            if match_not_found_list.check(match_id).await {
                println!("[{}] Match not found!", match_id);
                continue
            };

            match osu_api.get_match_all_events(match_id).await {
                Ok(data) => {
                    if data.is_match_disbanded() {
                        println!("Fetched {}", match_id);
                        //let boxed_data = Box::new(data);
                        //let _ = sender.send(boxed_data);
                    }
                }
                Err(e) => match e {
                    osu_api::error::OsuApiError::NotFound { .. } => {
                        match_not_found_list.insert(match_id).await;
                        println!("[{}] Inserted into not found", match_id);
                    }
                    osu_api::error::OsuApiError::TooManyRequests => {
                        panic!("TOO MANY REQUESTS")
                    }
                    _ => println!("[{}] Error during request: {e}", match_id),
                },
            }
        }
    }

    println!("Master exited");
}
