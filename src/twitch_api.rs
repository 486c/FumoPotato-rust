use reqwest::header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE};
use reqwest::{ Client, StatusCode, Method, Response, multipart };

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

#[derive(Deserialize, Debug)]
pub struct OuathResponse {
    access_token: String,
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
    client_secret: String,
    token: Option<String>,
}

impl TwitchApi {
    pub async fn new(client_id: &str, client_secret: &str) -> Result<Self> {
        let mut api = TwitchApi {
            client: Client::new(),
            token: None,
            client_secret: client_secret.to_string(),
            client_id: client_id.to_string(),
        };

        let token = api.request_oauth().await?;

        api.token = Some(token);
        Ok(api)
    }

    async fn make_request(&self, link: &str, method: Method) -> Result<Response> {

        let token = match &self.token { 
            Some(s) => s,
            None => return Err(eyre::Report::msg("No token found!"))
        };

        let r = &self.client;
        let r = match method {
            Method::GET => r.get(link),
            Method::POST => r.post(link),
            _ => unimplemented!(),
        };

        let r = r
            .header(ACCEPT, "application/json")
            .header(CONTENT_TYPE, "application/json")
            .header(AUTHORIZATION, format!("Bearer {}", token))
            .header("Client-Id", &self.client_id);

        Ok(r.send().await?)
    }

    async fn request_oauth(&self) -> Result<String> {
        let form = multipart::Form::new()
            .text("grant_type", "client_credentials")
            .text("client_id", self.client_id.clone())
            .text("client_secret", self.client_secret.clone());

        let r = self.client.post("https://id.twitch.tv/oauth2/token")
            .multipart(form)
            .header(ACCEPT, "application/json")
            .header(CONTENT_TYPE, "application/json")
            .header("Client-Id", &self.client_id)
            .send()
            .await?;

        let resp: OuathResponse = r.json().await?;

        Ok(resp.access_token)
    }

    pub async fn get_streams_by_name(
        &self, names: &[&str]
    ) -> Result<Option<Vec<TwitchStream>>> {
        let mut link = "https://api.twitch.tv/helix/streams?".to_owned();
        let mut users: Vec<TwitchStream> = Vec::with_capacity(names.len());

        if names.is_empty() {
            return Ok(None)
        };

        for chunk in names.chunks(100) {
            let mut iter = chunk.iter();
            
            // Probably should never ever fail
            let first = iter.next().unwrap();

            let _ = write!(link, "user_login={first}");

            for name in iter {
                let _ = write!(link, "&user_login={name}");
            }
            
            let r = self.make_request(&link, Method::GET).await?;

            let data = r.json::<TwitchResponse<TwitchStream>>().await?;

            if let Some(mut data) = data.data {
                users.append(&mut data);
            } else {
                return Ok(None)
            }
        };

        Ok(Some(users))
    }

    //TODO definitely use get_users_by_name instead
    // ^ look at get_streams_by_name as reference
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
}


#[cfg(test)]
mod tests {
    use crate::twitch_api::*;

    use std::env;
    use dotenv::dotenv;

    use eyre::Result;

    #[tokio::test]
    async fn test_get_user() -> Result<()> {
        dotenv().unwrap();

        let twitch_api = TwitchApi::new(
            env::var("TWITCH_CLIENT_ID").unwrap().as_str(),
            env::var("TWITCH_SECRET").unwrap().as_str()
        ).await?;

        let user = twitch_api.get_user_by_name("lopijb").await.unwrap();

        assert_eq!(145052794, user.id);

        Ok(())
    }
}

