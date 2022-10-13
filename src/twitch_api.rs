use reqwest::Client;
use reqwest::StatusCode;
use reqwest::header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE};

use anyhow::Result;

use serde::Deserialize;


#[derive(Deserialize, Debug, Clone)]
pub struct TwitchStream {
    id: String,
}

#[derive(Deserialize, Debug)]
pub struct TwitchResponse<T> {
    data: Option<Vec<T>>,
}

pub struct TwitchApi {
    client: Client,
    
    client_id: String,
    token: String,
}

impl TwitchApi {
    pub async fn init(token: &str, client_id: &str) -> Result<TwitchApi> {
        Ok(TwitchApi {
            client: Client::new(),
            token: token.to_string(),
            client_id: client_id.to_string(),
        })
    }
    
    pub async fn get_stream(&self, name: &str) -> Option<TwitchStream> {
        let link = format!("https://api.twitch.tv/helix/streams?user_login={}", name);

        let r = self
            .client
            .get(link)
            .header(ACCEPT, "application/json")
            .header(CONTENT_TYPE, "application/json")
            .header(AUTHORIZATION, format!("Bearer {}", self.token))
            .header("Client-Id", &self.client_id)
            .send()
            .await
            .unwrap();

        if r.status() != StatusCode::OK {
            println!("status code!");
            println!("{:?}", r.text().await);
            return None;
        }

        let s = r.json::<TwitchResponse<TwitchStream>>().await.unwrap();

        if let Some(data) = s.data {
            // There are no way that this gonna panic
            return Some(data[0].clone());
        } else {
            return None
        }

    }
}

#[cfg(test)]
mod tests {
    use crate::twitch_api::*;

    use std::env;
    use dotenv::dotenv;

    #[tokio::test]
    async fn test_get_stream() {
        dotenv().unwrap();

        let twitch_api = TwitchApi::init(
            env::var("TWITCH_TOKEN").unwrap().as_str(),
            env::var("TWITCH_CLIENT_ID").unwrap().as_str()
        ).await.unwrap();

        twitch_api.get_stream("melharucos").await.unwrap();
    }
}

