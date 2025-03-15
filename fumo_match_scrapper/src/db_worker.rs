use std::sync::Arc;

use fumo_database::Database;
use osu_api::models::osu_matches::OsuMatchGet;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio_util::sync::CancellationToken;

pub async fn worker(
    db: Arc<Database>,
    token: CancellationToken,
    mut receiver: UnboundedReceiver<Box<OsuMatchGet>>,
) {
    'main_loop: loop {
        if token.is_cancelled() {
            println!("Db worker exited");
            break;
        };

        let event = receiver.recv().await;
        if let Some(osu_match) = event {
            if let Some(end_time) = osu_match.osu_match.end_time {
                if db.insert_osu_match(
                    osu_match.osu_match.id,
                    &osu_match.osu_match.name,
                    osu_match.osu_match.start_time,
                    end_time,
                )
                .await
                .inspect_err(|e| {
                    println!("[{}] Failed to insert match into db: {e}", osu_match.osu_match.id)
                }).is_err() {
                    continue 'main_loop;
                };

                for event in osu_match.events {
                    if event.game.is_none() {
                        continue;
                    }

                    let game = &event.game.unwrap();

                    if db.insert_osu_match_game_from_request(
                        osu_match.osu_match.id,
                        game,
                    )
                    .await
                    .inspect_err(|e| {
                        println!("Failed to insert game into db: {e}")
                    }).is_err() {
                        continue 'main_loop;
                    };

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
                                    "[{}] Failed to insert game score into db: {e}",
                                    osu_match.osu_match.id
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
