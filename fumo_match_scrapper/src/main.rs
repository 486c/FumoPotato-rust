use clap::{command, Parser, Subcommand, ValueEnum};
use fumo_database::Database;
use osu_api::{models::osu_matches::OsuMatchGet, OsuApi};
use tokio::{signal, sync::mpsc::{self, UnboundedReceiver, UnboundedSender}};
use tokio_util::sync::CancellationToken;
use std::{env, ops::Range, sync::Arc, time::Duration};


#[derive(Subcommand, Debug)]
pub enum ScrapperKind {
    Range {
        /// A start match_id
        #[arg(short, long)]
        start: i64,

        /// A end match_id
        #[arg(short, long)]
        end: i64,
    },
    Linear {
        /// A match_id of the starting point (scrapper will go in new to old manner)
        #[arg(short, long)]
        start: i64,
    }
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// Amount of workers
    #[arg(short, long)]
    workers: usize,

    /// Batch sizeo of match_ids
    #[arg(short, long)]
    batch_size: usize,

    #[command(subcommand)]
    command: ScrapperKind,
}

async fn master(
    range: Range<i64>, 
    token: CancellationToken, 
    db: Arc<Database>,
    osu_api: Arc<OsuApi>,
    sender: UnboundedSender<Box<OsuMatchGet>>,
    batch_size: usize
) {
    let range: Vec<i64> = range.rev().collect();
    let mut to_process: Vec<i64> = Vec::new();

    for chunk in range.chunks(batch_size) { 
        if token.is_cancelled() {
            break;
        }

        // 1. Check in batch manner if match_ids in chunk are
        // exists or inside not found table
        let matches_result = 
            db.is_match_exists_and_not_found_batch(&chunk).await.unwrap();

        // 2. Clear to_process vec
        to_process.clear();
        
        // 3. Collect all neccessary match_ids
        for (k, v) in &matches_result {
            match v {
                (true, true) => {
                    println!("[{}] Match is expired, but it's in db", k);
                    continue
                },
                (true, false) => {
                    println!("[{}] Match exists, skipping", k);
                    continue
                },
                (false, true) => {
                    println!("[{}] Match not found", k);
                    continue
                },
                (false, false) => {},
            };

            to_process.push(*k);
        };

        for match_id in &to_process {
            match osu_api.get_match_all_events(*match_id).await {
                Ok(data) => {
                    if data.is_match_disbanded() {
                        println!("Fetched {}", match_id);
                        let boxed_data = Box::new(data);
                        let _ = sender.send(boxed_data);
                    }
                },
                Err(e) => {
                    match e {
                        osu_api::error::OsuApiError::NotFound { .. } => {
                            while let Err(e) = db.insert_osu_match_not_found(*match_id).await {
                                println!("Error inserting not found {e}");
                            }

                            println!("[{}] Inserted into not found", match_id);
                        },
                        osu_api::error::OsuApiError::TooManyRequests => panic!("TOO MANY REQUESTS"),
                        _ => println!("[{}] Error during request: {e}", match_id)
                    }
                }
            }
        }
    }

    println!("Master exited");
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
        true
    ).await.expect("Failed to initialize osu_api structure");

    let osu_api = Arc::new(osu_api);

    let cancel_token = CancellationToken::new();

    let chunks = match args.command {
        ScrapperKind::Range { start, end } => {
            
            let mut chunks = Vec::new();
            let chunk_size = (end - start) / args.workers as i64;
            let mut current_start = start;

            while current_start < end {
                let current_end = std::cmp::min(current_start + chunk_size, end);
                chunks.push(current_start..current_end);
                current_start += chunk_size;
            }

            chunks
        },
        ScrapperKind::Linear { start } => {
            let chunk_size = start / args.workers as i64;
            let mut chunks = Vec::new();
            let mut current_start = 0;
            while current_start < start {
                let end = (current_start + chunk_size).min(start);
                chunks.push(current_start..end);

                current_start = end;
            };

            chunks
        },
    };

    println!("Starting workers on chunks: {:?}", chunks);

    for chunk in chunks {
        tokio::spawn(master(chunk, cancel_token.clone(), db.clone(), osu_api.clone(), match_tx.clone(), args.batch_size));
    }

    tokio::spawn(db_worker(db.clone(), cancel_token.clone(), match_rx));

    tokio::select! {
        _ = signal::ctrl_c() => {
            cancel_token.cancel()
        },
    };

    tokio::time::sleep(Duration::from_secs(1)).await;
}