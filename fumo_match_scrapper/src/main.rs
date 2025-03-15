mod match_not_found;
mod scrap_worker;
mod db_worker;
mod live_scrapper;

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
    Live {
    }
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



#[tokio::main]
async fn main() {
    dotenv::dotenv().unwrap();

    let args = Args::parse();

    let (db_match_tx, db_match_rx) = mpsc::unbounded_channel();

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

    tokio::spawn(db_worker::worker(db.clone(), cancel_token.clone(), db_match_rx));

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
                    db_match_tx.clone(),
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
                    db_match_tx.clone(),
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
                db_match_tx.clone(),
                match_not_found_list.clone(),
                args.batch_size,
            ));
        },
        ScrapperKind::Live {} => {
            live_scrapper::run(
                osu_api.clone(),
                cancel_token.clone(),
                db_match_tx.clone(),
                db.clone()
            ).await;
        }
    };

    tokio::select! {
        _ = signal::ctrl_c() => {
            cancel_token.cancel()
        },
    };

    tokio::time::sleep(Duration::from_secs(1)).await;

    let _ = match_not_found_list.close().await;
}
