pub mod osu_api;
pub mod twitch_api;

use dotenv::dotenv;

use std::env;

use osu_api::OsuApi;
use twitch_api::TwitchApi;

#[tokio::main(worker_threads = 8)]
async fn main() {
    dotenv().unwrap();

    // Init twitch api
    let twitch_api = TwitchApi::init(
        env::var("TWITCH_TOKEN").unwrap().as_str(),
        env::var("TWITCH_CLIENT_ID").unwrap().as_str()
    ).await.unwrap();

    // Init osu api
    let osu_api = OsuApi::init(
        env::var("CLIENT_ID").unwrap().parse().unwrap(),
        env::var("CLIENT_SECRET").unwrap().as_str(),
        true
    ).await.unwrap();
    
    let token = env::var("DISCORD_TOKEN").unwrap();

    
    // Run
    /*
    tokio::select! {
        _ = client.start() => println!(""),
        res = signal::ctrl_c() => match res {
            Ok(_) => println!("\nGot Ctrl+C"),
            Err(_) => println!("Can't get Cntrl+C signal for some reason"),
        }
    }
    */

    println!("Bye!!!");
}
