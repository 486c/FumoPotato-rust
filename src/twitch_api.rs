use reqwest::header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE};
use reqwest::{ Client, StatusCode, Method, Response };

use eyre::Result;

use serde::Deserialize;

use serde::de::{ Visitor, Deserializer, Error };
use std::fmt::{ self, Write };

#[derive(Debug, Clone, Eq, PartialEq)]
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

fn str_to_i64<'de, D: Deserializer<'de>>(d: D) -> Result<i64, D::Error> {
    <&str as Deserialize>::deserialize(d)?
        .parse()
        .map_err(Error::custom)
}

#[derive(Deserialize, Debug, Clone)]
pub struct TwitchUser {
    #[serde(deserialize_with = "str_to_i64")]
    pub id: i64,
    pub login: String,
    pub display_name: String,
    //type
    pub broadcaster_type: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct TwitchStream {
    pub id: String,
    #[serde(deserialize_with = "str_to_i64")]
    pub user_id: i64,
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

    async fn make_request(&self, link: &str, method: Method) -> Result<Response> {
        let r = &self.client;
        let r = match method {
            Method::GET => r.get(link),
            Method::POST => r.post(link),
            _ => unimplemented!(),
        };

        let r = r
            .header(ACCEPT, "application/json")
            .header(CONTENT_TYPE, "application/json")
            .header(AUTHORIZATION, format!("Bearer {}", self.token))
            .header("Client-Id", &self.client_id);

        Ok(r.send().await?)
    }

    pub async fn get_streams_by_name(&self, names: &[&str]) -> Option<Vec<TwitchStream>> {
        let mut link = "https://api.twitch.tv/helix/streams?".to_owned();
        let mut users: Vec<TwitchStream> = Vec::with_capacity(names.len());

        if names.is_empty() {
            return None
        };

        for chunk in names.chunks(100) {
            let mut iter = chunk.iter();

            let first = iter.next()?;
            let _ = write!(link, "user_login={first}");

            for name in iter {
                let _ = write!(link, "&user_login={name}");
            }
            
            let r = self.make_request(&link, Method::GET).await;
            let r = match r {
                Ok(r) => r,
                Err(_) => return None,
            };

            let data = r.json::<TwitchResponse<TwitchStream>>().await.unwrap();
            if let Some(mut data) = data.data {
                users.append(&mut data);
            } else {
                return None
            }
        };

        Some(users)
    }

    pub async fn get_user_by_name(&self, name: &str) -> Option<TwitchUser> {
        let link = format!("https://api.twitch.tv/helix/users?login={}", name);

        let r = self.make_request(&link, Method::GET).await;

        let r = match r {
            Ok(r) => r,
            Err(_) => return None,
        };

        if r.status() != StatusCode::OK {
            return None;
        }

        let s = r.json::<TwitchResponse<TwitchUser>>().await.unwrap();

        if let Some(data) = s.data {
            Some(data.get(0)?.clone())
        } else {
            None
        }
    }

    /* TODO fetch streams by vector of id's instead of fetching one by one */
    pub async fn get_stream(&self, name: &str) -> Option<TwitchStream> {
        let link = format!("https://api.twitch.tv/helix/streams?user_login={}", name);

        let r = self.make_request(&link, Method::GET).await;

        // Handling worst case scenarios
        let r = match r {
            Ok(r) => r,
            Err(_) => return None,
        };

        if r.status() != StatusCode::OK {
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
                    user_id: Default::default(),
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
    async fn test_get_stream() {
        dotenv().unwrap();

        let twitch_api = TwitchApi::init(
            env::var("TWITCH_TOKEN").unwrap().as_str(),
            env::var("TWITCH_CLIENT_ID").unwrap().as_str()
        ).await.unwrap();

        let s = twitch_api.get_stream("ITMUSTFA11231ILASLDJKLSAKFJZMXCN123").await.unwrap();

        assert_eq!(s.stream_type, StreamType::Offline);
    }

    #[tokio::test]
    async fn test_get_user() {
        dotenv().unwrap();

        let twitch_api = TwitchApi::init(
            env::var("TWITCH_TOKEN").unwrap().as_str(),
            env::var("TWITCH_CLIENT_ID").unwrap().as_str()
        ).await.unwrap();

        let user = twitch_api.get_user_by_name("lopijb").await.unwrap();
        println!("{:?}", user);
    }
}

