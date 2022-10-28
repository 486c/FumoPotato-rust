use crate::config::BotConfig;

use reqwest::header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE};
use reqwest::Client;
use reqwest::StatusCode;

use crate::datetime::deserialize_local_datetime;

use chrono::prelude::*;

use anyhow::Result;

use serde::Deserialize;
use serde::de::{ Unexpected, Visitor, Deserializer, Error, SeqAccess };

use std::string::ToString;
use std::sync::Arc;
use std::fmt;
use std::time::Duration;

use tokio::sync::RwLock;

use bitflags::bitflags;

use tokio::sync::oneshot::{ channel, Receiver, Sender };

#[derive(Debug)]
pub enum OsuRank {
    GradeXH,
    GradeSH,
    GradeX,
    GradeS,
    GradeA,
    GradeB,
    GradeC,
    GradeD,
    GradeF,
}

impl OsuRank {
    pub fn to_emoji(&self) -> &str {
        match self {
            OsuRank::GradeXH => "<:r_XH:1004444329365999766>",
            OsuRank::GradeSH => "<:r_SH:1004444326669066270>",
            OsuRank::GradeX => "<:r_X:1004444328082538546>",
            OsuRank::GradeS => "<:r_S:1004444324840349759>",
            OsuRank::GradeA => "<:r_A:1004444322365702204>",
            OsuRank::GradeB => "<:r_B:1004444032149233696>",
            OsuRank::GradeC => "<:r_C:1004444033524957235>",
            OsuRank::GradeD => "<:r_D:1004444323703701545>",
            OsuRank::GradeF => "<:r_D:1004444323703701545>",
        }
    }
}

impl fmt::Display for OsuRank {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OsuRank::GradeXH => write!(f, "XH"),
            OsuRank::GradeSH => write!(f, "SH"),
            OsuRank::GradeX => write!(f, "X"),
            OsuRank::GradeS => write!(f, "S"),
            OsuRank::GradeA => write!(f, "A"),
            OsuRank::GradeB => write!(f, "B"),
            OsuRank::GradeC => write!(f, "C"),
            OsuRank::GradeD => write!(f, "D"),
            OsuRank::GradeF => write!(f, "F"),
        }
    }
}


struct OsuRankVisitor;

impl<'de> Visitor<'de> for OsuRankVisitor {
    type Value = OsuRank;

    #[inline]
    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a valid rank string")
    }

    fn visit_str<E: Error>(self, v: &str) -> Result<Self::Value, E> {
        let rank = match v {
            "XH" => OsuRank::GradeXH,
            "SH" => OsuRank::GradeSH,
            "X" => OsuRank::GradeX,
            "S" => OsuRank::GradeS,
            "A" => OsuRank::GradeA,
            "B" => OsuRank::GradeB,
            "C" => OsuRank::GradeC,
            "D" => OsuRank::GradeD,
            "F" => OsuRank::GradeF,
            _ => return Err(
                Error::invalid_value(
                    Unexpected::Str(v),
                    &r#""XH", "SH", "X", "S", "A", "B", "C", "D" or "F""#)
                ),
        };

        Ok(rank)
    }
}

impl<'de> Deserialize<'de> for OsuRank {
    #[inline]
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        d.deserialize_any(OsuRankVisitor)
    }
}

bitflags! {
    #[derive(Default)]
    pub struct OsuMods: u32 {
        const NOMOD = 0;
        const NOFAIL = 1;
        const EASY = 2;
        const TOUCHDEVICE = 4;
        const HIDDEN = 8;
        const HARDROCK = 16;
        const SUDDENDEATH = 32;
        const DOUBLETIME = 64;
        const RELAX = 128;
        const HALFTIME = 256;
        const NIGHTCORE = 512 | Self::DOUBLETIME.bits;
        const FLASHLIGHT = 1024;
        const SPUNOUT = 4096;
        const PERFECT = 16_384 | Self::SUDDENDEATH.bits;
        const FADEIN = 1_048_576;
        const SCOREV2 = 536_870_912;
        const MIRROR = 1_073_741_824;
    }
}

impl ToString for OsuMods {
    fn to_string(&self) -> String {
        let mut res = String::new();

        if self.bits == 0 {
            res.push_str("NM");
            return res
        }

        if self.contains(OsuMods::NOFAIL) {
            res.push_str("NF")
        }
        if self.contains(OsuMods::EASY) {
            res.push_str("EZ")
        }
        if self.contains(OsuMods::TOUCHDEVICE) {
            res.push_str("TD")
        }
        if self.contains(OsuMods::HIDDEN) {
            res.push_str("HD")
        }
        if self.contains(OsuMods::DOUBLETIME) {
            if self.contains(OsuMods::NIGHTCORE) {
                res.push_str("NC")
            } else {
                res.push_str("DT")
            }
        }
        if self.contains(OsuMods::HALFTIME) {
            res.push_str("HT")
        }
        if self.contains(OsuMods::FLASHLIGHT) {
            res.push_str("FL")
        }
        if self.contains(OsuMods::HARDROCK) {
            res.push_str("HR")
        }
        if self.contains(OsuMods::SUDDENDEATH) {
            res.push_str("SD")
        }
        if self.contains(OsuMods::SPUNOUT) {
            res.push_str("SO")
        }
        if self.contains(OsuMods::PERFECT) {
            res.push_str("PF")
        }
        if self.contains(OsuMods::MIRROR) {
            res.push_str("MR")
        }

        res
    }
}

struct OsuModsVisitor;

impl<'de> Visitor<'de> for OsuModsVisitor {
    type Value = OsuMods;

    #[inline]
    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("")
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let mut mods = OsuMods::default();

        while let Some(next) = seq.next_element()? {
            mods |= next;
        }

        Ok(mods)
    }

    fn visit_str<E: Error>(self, v: &str) -> Result<Self::Value, E> {
        let mods = match v {
            "NM" => OsuMods::NOMOD,
            "NF" => OsuMods::NOFAIL,
            "EZ" => OsuMods::EASY,
            "TD" => OsuMods::TOUCHDEVICE,
            "HD" => OsuMods::HIDDEN,
            "HR" => OsuMods::HARDROCK,
            "SD" => OsuMods::SUDDENDEATH,
            "DT" => OsuMods::DOUBLETIME,
            "RX" => OsuMods::RELAX,
            "HT" => OsuMods::HALFTIME,
            "NC" => OsuMods::NIGHTCORE,
            "FL" => OsuMods::FLASHLIGHT,
            "SO" => OsuMods::SPUNOUT,
            "PF" => OsuMods::PERFECT,
            "FD" => OsuMods::FADEIN,
            _ => return Err(
                Error::invalid_value(
                    Unexpected::Str(v),
                    &r#"valid mods acronym"#)
                ),
        };

        Ok(mods)
    }
}

impl<'de> Deserialize<'de> for OsuMods {
    #[inline]
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        d.deserialize_any(OsuModsVisitor)
    }
}

#[derive(Deserialize, Debug)]
struct OauthResponse {
    token_type: String,
    expires_in: i32,
    access_token: String,
}

#[derive(Deserialize, Debug)]
pub struct OsuScoreStatistics {
    #[serde(rename = "count_50")]
    pub count50: i32,
    #[serde(rename = "count_100")]
    pub count100: i32,
    #[serde(rename = "count_300")]
    pub count300: i32,
    #[serde(rename = "count_geki")]
    pub countgeki: i32,
    #[serde(rename = "count_katu")]
    pub countkatu: i32,
    #[serde(rename = "count_miss")]
    pub countmiss: i32,
}

#[derive(Deserialize, Debug)]
pub struct OsuBeatmapsetCompact {
    title: String,
    artist: String,
    creator: String,
}

#[derive(Deserialize, Debug)]
pub struct OsuBeatmap {
    pub beatmapset_id: i32,
    pub id: i32,
    pub mode: String,

    pub version: String,

    pub beatmapset: OsuBeatmapsetCompact,

    pub max_combo: i32,
}

impl OsuBeatmap {
    pub fn metadata(&self) -> String {
        format!("{} - {} [{}]", self.beatmapset.artist, self.beatmapset.title, self.version)
    }
}

#[derive(Deserialize, Debug)]
pub struct OsuScore {
    pub id: i64,
    pub best_id: i64,
    pub user_id: i64,
    pub accuracy: f32,
    pub mods: OsuMods,
    pub score: i64,
    pub perfect: bool,
    pub passed: bool,
    pub pp: Option<f32>,

    #[serde(rename = "max_combo")]
    pub max_combo: i32,

    pub rank: OsuRank,

    #[serde(deserialize_with = "deserialize_local_datetime")]
    pub created_at: DateTime<Utc>,

    #[serde(rename = "statistics")]
    pub stats: OsuScoreStatistics,

    pub mode: String,
    pub mode_int: i16,
    pub replay: bool,
    pub user: OsuUserCompact,
}

#[derive(Deserialize, Debug)]
pub struct OsuLeaderboard {
    pub scores: Vec<OsuScore>,
}

#[derive(Deserialize, Debug)]
pub struct OsuUserCompact {
    pub avatar_url: String,
    pub country_code: String,
    pub default_group: String,
    pub id: i64,
    pub is_active: bool,
    pub is_bot: bool,
    pub is_deleted: bool,
    pub is_online: bool,
    pub is_supporter: bool,
    pub pm_friends_only: bool,
    pub username: String,
    // last_visit & profile_colour skipped
}

#[derive(Debug)]
pub struct OsuToken {
    client: Client,

    client_id: i32,
    secret: String,
    token: RwLock<String>,
}

#[derive(Debug)]
pub struct OsuApi {
    inner: Arc<OsuToken>,
    loop_drop_tx: Option<Sender<()>>,
}

impl Drop for OsuApi {
    fn drop(&mut self) {
        if let Some(tx) = self.loop_drop_tx.take() {
            let _ = tx.send(());
        }
    }
}


impl OsuToken {
    async fn request_oauth(&self) -> Result<OauthResponse> {
        let data = format!(
            r#"{{
            "client_id":"{}",
            "client_secret":"{}",
            "grant_type":"client_credentials",
            "scope":"public" 
        }}"#,
            &self.client_id, &self.secret
        );

        let r = self
            .client
            .post("https://osu.ppy.sh/oauth/token")
            .body(data.into_bytes())
            .header(ACCEPT, "application/json")
            .header(CONTENT_TYPE, "application/json")
            .send()
            .await
            .unwrap();

        let json_data = r.json::<OauthResponse>().await?;

        Ok(json_data)
    }
}

impl OsuApi {

    pub async fn get_beatmap(&self, bid: i32) -> Option<OsuBeatmap> {
        let link = format!("https://osu.ppy.sh/api/v2/beatmaps/{}", bid);
        let token = self.inner.token.read().await;

        let r = self.inner
            .client
            .get(link)
            .header(ACCEPT, "application/json")
            .header(CONTENT_TYPE, "application/json")
            .header(AUTHORIZATION, format!("Bearer {}", token))
            .send()
            .await
            .unwrap();

        if r.status() != StatusCode::OK {
            return None;
        }
        
        let b = r.json::<OsuBeatmap>().await.unwrap();

        Some(b)
    }

    // This method works only if FALLBACK_API variable
    // is set.
    pub async fn get_countryleaderboard(&self, bid: i32) -> Option<OsuLeaderboard> {
        let cfg = match BotConfig::get_res() {
            Some(c) => c,
            None => return None,
        };

        let link = format!(
            "{}/leaderboard/leaderboard?beatmap={}&type=country",
            cfg.fallback_api,
            bid
        );

        let token = self.inner.token.read().await;

        let r = self.inner
            .client
            .get(link)
            .header(ACCEPT, "application/json")
            .header(CONTENT_TYPE, "application/json")
            .header(AUTHORIZATION, format!("Bearer {}", token))
            .send()
            .await
            .unwrap();

        if r.status() != StatusCode::OK {
            return None;
        }

        let b = r.json::<OsuLeaderboard>().await.unwrap();

        Some(b)
    }
}

impl OsuApi {
    pub async fn init(client_id: i32, secret: &str) -> Result<OsuApi> {
        let inner = Arc::new(OsuToken {
            client: Client::new(),
            client_id,
            secret: secret.to_string(),
            token: Default::default(),
        });

        let response = inner.request_oauth().await.unwrap();
        let mut token = inner.token.write().await;
        *token = response.access_token;
        drop(token);

        let (tx, rx) = channel::<()>();

        OsuApi::update_token(
            Arc::clone(&inner), 
            response.expires_in as u64,
            rx
        ).await;

        let api = OsuApi {
            loop_drop_tx: Some(tx),
            inner,
        };

        Ok(api)
    }

    async fn update_token(osu: Arc<OsuToken>, expire: u64, rx: Receiver<()>) {
        tokio::spawn(async move {
            OsuApi::token_loop(Arc::clone(&osu), expire, rx).await;
            println!("osu!api token loop is closed!");
        });
    }

    async fn token_loop(osu: Arc<OsuToken>, mut expire: u64, mut rx: Receiver<()>) {
        loop {
            expire /= 2;
            println!("Token update scheduled in {} seconds", expire);
            tokio::select!{
                _ = tokio::time::sleep(Duration::from_secs(expire)) => {}
                _ = &mut rx => {
                    return;
                }
            }

            let response = osu.request_oauth().await.unwrap();

            let mut token = osu.token.write().await;
            *token = response.access_token;

            expire = response.expires_in as u64;
            println!("Successfully updated osu! token!");
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::osu_api::*;

    use std::env;
    use dotenv::dotenv;


    #[tokio::test]
    async fn test_something() {
        dotenv().unwrap();

        let api = OsuApi::init(
            env::var("CLIENT_ID").unwrap().parse().unwrap(),
            env::var("CLIENT_SECRET").unwrap().as_str(),
        ).await.unwrap();

        let mut op = api.get_beatmap(3153603).await;

        assert!(!op.is_none());

        let b = op.unwrap();
        assert_eq!(b.id, 3153603);

        op = api.get_beatmap(12).await;
        assert!(op.is_none());
    }

    #[test]
    fn mods_test() {
        let mods = OsuMods::NOMOD;
        assert_eq!(mods.to_string(), "NM");

        let mods = OsuMods::NOMOD | OsuMods::HIDDEN;
        assert_eq!(mods.to_string(), "HD");

        let mods = OsuMods::HARDROCK | OsuMods::HIDDEN;
        assert_eq!(mods.to_string(), "HDHR");

        let mods = OsuMods::DOUBLETIME | OsuMods::HIDDEN;
        assert_eq!(mods.to_string(), "HDDT");

        let mods = OsuMods::NIGHTCORE;
        assert_eq!(mods.to_string(), "NC");

        let mods = OsuMods::NIGHTCORE | OsuMods::HIDDEN;
        assert_eq!(mods.to_string(), "HDNC");

        let mods = OsuMods::NIGHTCORE | OsuMods::HIDDEN | OsuMods::HARDROCK;
        assert_eq!(mods.to_string(), "HDNCHR");
    }
}
