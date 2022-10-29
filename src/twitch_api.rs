use reqwest::Client;
use reqwest::StatusCode;
use reqwest::header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE};

use anyhow::Result;

use serde::Deserialize;

use serde::de::{ Visitor, Deserializer, Error };
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum StreamType {
    Live,
    Offline,
}

struct StreamTypeVisitor;

impl<'de> Visitor<'de> for StreamTypeVisitor {
    type Value = StreamType;

    #[inline]
    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a valid stream type string")
    }


    fn visit_str<E: Error>(self, v: &str) -> Result<Self::Value, E> {
        match v {
            "live" => Ok(StreamType::Live),
            _ => Ok(StreamType::Offline),
        }
    }
}

impl<'de> Deserialize<'de> for StreamType {
    #[inline]
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        d.deserialize_any(StreamTypeVisitor)
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct TwitchStream {
    pub id: String,
    pub user_login: String,
    pub user_name: String,
    pub game_name: String,
    pub game_id: String,
    pub title: String,

    #[serde(rename = "type")] 
    pub stream_type: StreamType,
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
            .await;
    
        // Handling worst case scenarios
        let r = match r {
            Ok(r) => r,
            Err(_) => return None,
        };

        if r.status() != StatusCode::OK {
            println!("status code!");
            println!("{:?}", r.text().await);
            return None;
        }

        
        let s = r.json::<TwitchResponse<TwitchStream>>().await.unwrap();

        if let Some(data) = s.data {

            // Since twitch is returning just empty data instead of saying if streamer is online or not
            // so we assuming that empty data = stream is offline 
            if let Some(stream) = data.get(0) {
                Some(stream.clone())
            } 
            else {
                Some(TwitchStream {
                    id: Default::default(),
                    user_login: Default::default(),
                    user_name: Default::default(),
                    game_name: Default::default(),
                    game_id: Default::default(),
                    stream_type: StreamType::Offline,
                    title: Default::default(),
                })
            }

        } 
        else {
            None
        }

    }
}

#[cfg(test)]
mod tests {
    use crate::twitch_api::*;

    use std::env;
    use dotenv::dotenv;

    #[tokio::test]
    #[should_panic]
    async fn test_get_stream() {
        dotenv().unwrap();

        let twitch_api = TwitchApi::init(
            env::var("TWITCH_TOKEN").unwrap().as_str(),
            env::var("TWITCH_CLIENT_ID").unwrap().as_str()
        ).await.unwrap();

        twitch_api.get_stream("ITMUSTFAILASLDJKLSAKFJZMXCN123").await.unwrap();
    }
}

