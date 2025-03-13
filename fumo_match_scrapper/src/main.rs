mod match_not_found;
mod scrap_worker;

use clap::{command, Parser, Subcommand};
use fumo_database::Database;
use match_not_found::MatchNotFoundList;
use osu_api::{models::osu_matches::OsuMatchGet, OsuApi};
use std::{
    env,
    fs::File,
    io::{BufRead, BufReader},
    path::PathBuf,
    sync::Arc,
    time::Duration,
};
use tokio::{
    signal,
    sync::mpsc::{self, UnboundedReceiver},
};
use tokio_util::sync::CancellationToken;

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
    },
    File {
        /// A file with match_id separated by new-line
        #[arg(short, long)]
        file: PathBuf,
    },
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// Amount of workers
    #[arg(short, long)]
    workers: usize,

    /// Batch size of the match ids
    #[arg(short, long)]
    batch_size: usize,

    #[command(subcommand)]
    command: ScrapperKind,
}


async fn db_worker(
    db: Arc<Database>,
    token: CancellationToken,
    mut receiver: UnboundedReceiver<Box<OsuMatchGet>>,
) {
    loop {
        if token.is_cancelled() {
            println!("Db worker exited");
            break;
        };

        let event = receiver.recv().await;
        if let Some(osu_match) = event {
            if let Some(end_time) = osu_match.osu_match.end_time {
                let _ = db
                    .insert_osu_match(
                        osu_match.osu_match.id,
                        &osu_match.osu_match.name,
                        osu_match.osu_match.start_time,
                        end_time,
                    )
                    .await
                    .inspect_err(|e| {
                        println!("Failed to insert match into db: {e}")
                    });

                for event in osu_match.events {
                    if event.game.is_none() {
                        continue;
                    }

                    let game = &event.game.unwrap();

                    let _ = db
                        .insert_osu_match_game_from_request(
                            osu_match.osu_match.id,
                            game,
                        )
                        .await
                        .inspect_err(|e| {
                            println!("Failed to insert game into db: {e}")
                        });

                    for score in &game.scores {
                        let _ = db
                            .insert_osu_match_game_score_from_request(
                                osu_match.osu_match.id,
                                game.id,
                                game.beatmap_id,
                                score,
                            )
                            .await
                            .inspect_err(|e| {
                                println!(
                                    "Failed to insert game score into db: {e}"
                                )
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

    let db = Database::init(env::var("DATABASE_URL").unwrap().as_str())
        .await
        .expect("Failed to initialize database connection");

    let db = Arc::new(db);

    let osu_api = OsuApi::new(
        env::var("CLIENT_ID").unwrap().parse().unwrap(),
        env::var("CLIENT_SECRET").unwrap().as_str(),
        env::var("OSU_SESSION").unwrap().as_str(),
        env::var("FALLBACK_API").unwrap().as_str(),
        env::var("FALLBACK_API_KEY").unwrap().as_str(),
        true,
    )
    .await
    .expect("Failed to initialize osu_api structure");

    let osu_api = Arc::new(osu_api);

    let cancel_token = CancellationToken::new();
    let match_not_found_list = Arc::new(MatchNotFoundList::new().unwrap());

    match args.command {
        ScrapperKind::Range { start, end } => {
            let mut chunks = Vec::new();
            let chunk_size = (end - start) / args.workers as i64;
            let mut current_start = start;

            while current_start < end {
                let current_end =
                    std::cmp::min(current_start + chunk_size, end);
                chunks.push(current_start..current_end);
                current_start += chunk_size;
            }

            println!("Range: Starting workers on chunks: {:?}", chunks);
            for chunk in chunks {
                tokio::spawn(scrap_worker::run(
                    chunk.collect(),
                    cancel_token.clone(),
                    db.clone(),
                    osu_api.clone(),
                    match_tx.clone(),
                    match_not_found_list.clone(),
                    args.batch_size,
                ));
            }
        }
        ScrapperKind::Linear { start } => {
            let chunk_size = start / args.workers as i64;
            let mut chunks = Vec::new();
            let mut current_start = 0;
            while current_start < start {
                let end = (current_start + chunk_size).min(start);
                chunks.push(current_start..end);

                current_start = end;
            }

            println!("Linear: Starting workers on chunks: {:?}", chunks);
            for chunk in chunks {
                tokio::spawn(scrap_worker::run(
                    chunk.collect(),
                    cancel_token.clone(),
                    db.clone(),
                    osu_api.clone(),
                    match_tx.clone(),
                    match_not_found_list.clone(),
                    args.batch_size,
                ));
            }
        }
        ScrapperKind::File { file } => {
            let file = BufReader::new(File::open(file).unwrap());

            let mut chunk: Vec<i64> = Vec::new();

            for line in file.lines() {
                let line = line.unwrap();

                if line.is_empty() || &line == "\n" {
                    continue;
                }

                if let Ok(match_id) = line.parse() {
                    chunk.push(match_id);
                } else {
                    println!("Failed to parse match id from file")
                }
            }

            tokio::spawn(scrap_worker::run(
                chunk,
                cancel_token.clone(),
                db.clone(),
                osu_api.clone(),
                match_tx.clone(),
                match_not_found_list.clone(),
                args.batch_size,
            ));
        }
    };

    tokio::spawn(db_worker(db.clone(), cancel_token.clone(), match_rx));

    tokio::select! {
        _ = signal::ctrl_c() => {
            cancel_token.cancel()
        },
    };

    tokio::time::sleep(Duration::from_secs(1)).await;

    let _ = match_not_found_list.close().await;
}
