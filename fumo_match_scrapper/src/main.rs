use clap::{command, Parser};
use fumo_database::Database;
use osu_api::{models::osu_matches::OsuMatchGet, OsuApi};
use tokio::{signal, sync::mpsc::{self, UnboundedReceiver, UnboundedSender}};
use tokio_util::sync::CancellationToken;
use std::{env, ops::Range, sync::Arc, time::Duration};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// A match_id of the starting point (scrapper will go in new to old manner)
    #[arg(short, long)]
    start: i64,
    
    /// Amount of workers
    #[arg(short, long)]
    workers: usize
}


async fn master(
    range: Range<i64>, 
    token: CancellationToken, 
    db: Arc<Database>,
    osu_api: Arc<OsuApi>,
    sender: UnboundedSender<Box<OsuMatchGet>>
) {

    for current in range.rev() {
        if token.is_cancelled() {
            println!("Stopped master");
            break
        };

        let is_match_exists = 'blk: {
            loop {
                match db.is_osu_match_exists(current).await {
                    Ok(n) => break 'blk n,
                    Err(e) => {
                        println!("Error db: {e}");
                        continue;
                    }
                }
            }
        };

        let is_match_not_found = 'blk: {
            loop {
                match db.is_osu_match_not_found(current).await {
                    Ok(n) => break 'blk n,
                    Err(e) => {
                        println!("Error db: {e}");
                        continue;
                    }
                }
            }
        };

        match (is_match_exists, is_match_not_found) {
            (true, true) => {
                println!("[{}] Match is expired, but it's in db", current);
                continue
            },
            (true, false) => {
                println!("[{}] Match exists, skipping", current);
                continue
            },
            (false, true) => {
                println!("[{}] Match not found", current);
                continue
            },
            (false, false) => {},
        };

        if is_match_exists {
            println!("[{}] Match exists, skipping...", current);
            continue;
        }

        match osu_api.get_match_all_events(current).await {
            Ok(data) => {
                if data.is_match_disbanded() {
                    println!("Fetched {}", current);
                    let boxed_data = Box::new(data);
                    let _ = sender.send(boxed_data);
                }
            },
            Err(e) => {
                match e {
                    osu_api::error::OsuApiError::NotFound { .. } => {
                        while let Err(e) = db.insert_osu_match_not_found(current).await {
                            println!("Error inserting not found {e}");
                        }

                        println!("[{}] Inserted into not found", current);
                    },
                    osu_api::error::OsuApiError::TooManyRequests => panic!("TOO MANY REQUESTS"),
                    _ => println!("[{}] Error during request: {e}", current)
                }
            }
        }
    }
}

async fn db_worker(
    db: Arc<Database>,
    token: CancellationToken,
    mut receiver: UnboundedReceiver<Box<OsuMatchGet>>
) {
    loop {
        if token.is_cancelled() {
            println!("Db worker exited");
            break;
        };

        let event = receiver.recv().await;
        if let Some(osu_match) = event {
            if let Some(end_time) = osu_match.osu_match.end_time {
                let _ = db.insert_osu_match(
                    osu_match.osu_match.id,
                    &osu_match.osu_match.name,
                    osu_match.osu_match.start_time,
                    end_time,
                ).await.inspect_err(|e| {
                    println!("Failed to insert match into db: {e}")
                });

                for event in osu_match.events {
                    if event.game.is_none() {
                        continue;
                    }

                    let game = &event.game.unwrap();

                    let _ = db.insert_osu_match_game_from_request(
                        osu_match.osu_match.id,
                        &game
                    ).await.inspect_err(|e| {
                        println!("Failed to insert game into db: {e}")
                    });

                    for score in &game.scores {
                        let _ = db.insert_osu_match_game_score_from_request(
                            osu_match.osu_match.id,
                            game.id,
                            game.beatmap_id,
                            &score
                        ).await.inspect_err(|e| {
                            println!("Failed to insert game score into db: {e}")
                        });
                    }
                }
            } else {
                println!("[DB_WORKER] Got match to insert without end_time!")
            }


            println!("Inserted into DB: current queue => {}", receiver.len());
        } else {
            println!("DB Worker is in bad state TODO");
        }

    }
}

#[tokio::main]
async fn main() {
    dotenv::dotenv().unwrap();

    let args = Args::parse();

    let (match_tx, match_rx) = mpsc::unbounded_channel();

    let db = Database::init(
        env::var("DATABASE_URL").unwrap().as_str()
    ).await.expect("Failed to initialize database connection");

    let db = Arc::new(db);

    let osu_api = OsuApi::new(
        env::var("CLIENT_ID").unwrap().parse().unwrap(),
        env::var("CLIENT_SECRET").unwrap().as_str(),
        env::var("OSU_SESSION").unwrap().as_str(),
        env::var("FALLBACK_API").unwrap().as_str(),
        false
    ).await.expect("Failed to initialize osu_api structure");

    let osu_api = Arc::new(osu_api);

    let cancel_token = CancellationToken::new();

    let chunk_size = args.start / args.workers as i64;
    let mut chunks = Vec::new();
    let mut start = 0;
    while start < args.start {
        let end = (start + chunk_size).min(args.start);
        chunks.push(start..end);

        start = end;
    }

    for chunk in chunks {
        tokio::spawn(master(chunk, cancel_token.clone(), db.clone(), osu_api.clone(), match_tx.clone()));
    }

    tokio::spawn(db_worker(db.clone(), cancel_token.clone(), match_rx));

    tokio::select! {
        _ = signal::ctrl_c() => {
            cancel_token.cancel()
        },
    };

    tokio::time::sleep(Duration::from_secs(1)).await;
}
