use reqwest::header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE};
use reqwest::{ Client, Method, Response, multipart };

use eyre::Result;

use serde::Deserialize;

use serde::de::{ Visitor, Deserializer, Error, DeserializeOwned };
use tokio::sync::RwLock;
use tokio::sync::oneshot::{ channel, Receiver, Sender };
use std::fmt::{ self, Write };
use std::sync::Arc;
use std::time::Duration;

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

#[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
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
    expires_in: u64,
}

#[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct TwitchStream {
    pub id: String,
    #[serde(deserialize_with = "str_to_i64")]
    pub user_id: i64,
    pub user_login: String,
    pub user_name: String,
    pub game_name: String,
    pub game_id: String, // TODO use i64
    pub title: String,

    #[serde(rename = "type")] 
    pub stream_type: StreamType,
}

#[derive(Deserialize, Debug)]
pub struct TwitchResponse<T> {
    data: Option<Vec<T>>,
}

pub struct TwitchToken {
    client: Client,
    client_id: String,
    client_secret: String,

    token: RwLock<Option<String>>,
}

impl TwitchToken {
    async fn request_oauth(&self) -> Result<OuathResponse> {
        let form = multipart::Form::new()
            .text("grant_type", "client_credentials")
            .text("client_id", self.client_id.clone())
            .text("client_secret", self.client_secret.clone());

        let r = self.client.post(
            "https://id.twitch.tv/oauth2/token"
            )
            .multipart(form)
            .header(ACCEPT, "application/json")
            .header(CONTENT_TYPE, "application/json")
            .header("Client-Id", &self.client_id)
            .send()
            .await?;

        let resp: OuathResponse = r.json().await?;

        Ok(resp)
    }
}

pub struct TwitchApi {
    inner: Arc<TwitchToken>,
    sender: Option<Sender<()>>,
}

impl Drop for TwitchApi {
    fn drop(&mut self) {
        if let Some(tx) = self.sender.take() {
            let _ = tx.send(());
        }
    }
}

impl TwitchApi {
    pub async fn new(
        client_id: &str, 
        client_secret: &str
    ) -> Result<Self> {
        let client = Client::builder()
            .https_only(true)
            .use_native_tls()
            .build()?;

        let inner = TwitchToken {
            client,
            client_id: client_id.to_owned(),
            client_secret: client_secret.to_owned(),
            token: None.into(),
        };

        let token = inner.request_oauth().await?;

        let (tx, rx) = channel::<()>();

        let mut lock = inner.token.write().await;

        *lock = Some(token.access_token);

        drop(lock);

        let inner = Arc::new(inner);

        let api = TwitchApi {
            sender: Some(tx),
            inner: Arc::clone(&inner),
        };

        TwitchApi::update_token(
            Arc::clone(&inner),
            token.expires_in,
            rx
        ).await;

        Ok(api)
    }

    async fn update_token(
        inner: Arc<TwitchToken>,
        mut expire: u64,
        mut rx: Receiver<()>
    ) {
        tokio::spawn(async move {
            loop {
                expire /= 2;
                println!("Twitch token update scheduled in {expire} seconds");
                tokio::select!{
                    _ = tokio::time::sleep(Duration::from_secs(expire)) => {}
                    _ = &mut rx => {
                        println!("twitch token loop is closed!");
                        break;
                    }
                }

                let response = inner.request_oauth().await.unwrap();
                
                let mut token = inner.token.write().await;
                *token = Some(response.access_token);

                expire = response.expires_in;
                println!("Successfully updated twitch api token");
            }

        });
    }

    pub async fn download_image(&self, link: &str) -> Result<Vec<u8>> {
        let r = self.inner.client.get(link)
            .header(ACCEPT, "image/jpeg")
            .header("Cache-Control", "no-cache")
            .header("User-Agent", "fumo_potato")
            .send().await?;

        let bytes = r.bytes().await?;

        Ok(bytes.to_vec())
    }

    async fn make_request(
        &self, 
        link: &str, 
        method: Method
    ) -> Result<Response> {

        let lock = &self.inner.token.read().await;

        let token = match lock.as_ref() { 
            Some(s) => s,
            None => return Err(eyre::Report::msg("No token found!"))
        };

        let r = &self.inner.client;
        let r = match method {
            Method::GET => r.get(link),
            Method::POST => r.post(link),
            _ => unimplemented!(),
        };

        let r = r
            .header(ACCEPT, "application/json")
            .header(CONTENT_TYPE, "application/json")
            .header(AUTHORIZATION, format!("Bearer {token}"))
            .header("Client-Id", &self.inner.client_id);
        
        // Check for errors?
        Ok(r.send().await?)
    }

    
    async fn request_list<T: DeserializeOwned, U: std::fmt::Display>(
        &self,
        link: &str,
        separator: &str,
        items: &[U],
    ) -> Result<Option<Vec<T>>> {
        let mut link = link.to_owned();

        if items.is_empty() {
            return Ok(None)
        };

        let mut out_items: Vec<T> = Vec::with_capacity(items.len());

        for chunk in items.chunks(100) {
            let mut iter = chunk.iter();

            let first = iter.next().unwrap();
            let _ = write!(link, "?{separator}={first}");

            for i in iter {
                let _ = write!(link, "&{separator}={i}");
            }

            let r = self.make_request(&link, Method::GET).await?;

            let data = r.json::<TwitchResponse<T>>().await?;

            if let Some(mut data) = data.data {
                out_items.append(&mut data);
            } else {
                return Ok(None)
            }
        };

        Ok(Some(out_items))
    }

    pub async fn get_streams_by_name(
        &self, names: &[&str]
    ) -> Result<Option<Vec<TwitchStream>>> {
        self.request_list(
            "https://api.twitch.tv/helix/streams",
            "user_login",
            names
        ).await
    }

    pub async fn get_streams_by_id(
        &self, ids: &[i64]
    ) -> Result<Option<Vec<TwitchStream>>> {
        self.request_list(
            "https://api.twitch.tv/helix/streams",
            "user_id",
            ids
        ).await
    }

    pub async fn get_users_by_name(
        &self, 
        names: &[&str]
    ) -> Result<Option<Vec<TwitchUser>>> {
        self.request_list(
            "https://api.twitch.tv/helix/users",
            "login",
            names
        ).await
    }

    pub async fn get_users_by_id(
        &self, 
        ids: &[i64]
    ) -> Result<Option<Vec<TwitchUser>>> {
        self.request_list(
            "https://api.twitch.tv/helix/users",
            "id",
            ids
        ).await
    }
}

#[cfg(test)]
mod tests {
    use crate::twitch_api::*;

    use std::env;
    use dotenv::dotenv;

    use eyre::Result;
    use async_once_cell::OnceCell;

    static API_INSTANCE: OnceCell<TwitchApi> = OnceCell::new();

    async fn get_api() -> &'static TwitchApi {
        dotenv().unwrap();
        
        API_INSTANCE.get_or_init(async {
            TwitchApi::new(
                env::var("TWITCH_CLIENT_ID").unwrap().as_str(),
                env::var("TWITCH_SECRET").unwrap().as_str()
            )
            .await
            .expect("Failed to initialize twitch api")
        }).await
    }

    #[tokio::test]
    async fn test_get_users_non_existent_user() -> Result<()> {
        let twitch_api = get_api().await;

        let list = twitch_api.get_users_by_name(
            &["bebrikkakawka123"]
        ).await?.unwrap();

        assert_eq!(list.get(0), None);
        
        Ok(())
    }

    #[tokio::test]
    async fn test_get_streams_by_id() -> Result<()> {
        let twitch_api = get_api().await;

        let mut list = twitch_api.get_users_by_id(
            &[145052794, 12826, 544067520]
        ).await?.unwrap();
        
        list.sort_by_key(|s| s.id);

        let expected: Vec<TwitchUser> = vec![
            TwitchUser {
                id: 12826,
                login: "twitch".to_owned(),
                display_name: "Twitch".to_owned(),
                broadcaster_type: "partner".to_owned(),
            },
            TwitchUser {
                id: 145052794,
                login: "lopijb".to_owned(),
                display_name: "バカです".to_owned(),
                broadcaster_type: "".to_owned(),
            },
            TwitchUser {
                id: 544067520,
                login: "dados123osu".to_owned(),
                display_name: "dados123osu".to_owned(),
                broadcaster_type: "".to_owned(),
            },
        ];

        assert_eq!(list, expected);
        Ok(())
    }

    #[tokio::test]
    async fn test_get_streams_by_name() -> Result<()> {
        let twitch_api = get_api().await;

        let mut list = twitch_api.get_users_by_name(
            &["lopijb", "twitch"]
        ).await?.unwrap();

        list.sort_by_key(|s| s.id);

        let expected: Vec<TwitchUser> = vec![
            TwitchUser {
                id: 12826,
                login: "twitch".to_owned(),
                display_name: "Twitch".to_owned(),
                broadcaster_type: "partner".to_owned(),
            },
            TwitchUser {
                id: 145052794,
                login: "lopijb".to_owned(),
                display_name: "バカです".to_owned(),
                broadcaster_type: "".to_owned(),
            },
        ];

        assert_eq!(list, expected);
        Ok(())
    }

    #[tokio::test]
    async fn test_get_image() -> Result<()> {
        let twitch_api = get_api().await;

        let image_bytes = twitch_api
            .download_image("https://static-cdn.jtvnw.net/ttv-boxart/21465_IGDB-188x250.jpg").await?;

        dbg!(image_bytes.len());

        Ok(())
    }
}

