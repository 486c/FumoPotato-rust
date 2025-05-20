mod datetime;
mod datetime_timestamp;
mod metrics;

pub mod error;
pub mod models;
pub mod fallback_models;

use fallback_models::FallbackBeatmapScores;
use models::{osu_matches::{OsuMatchContainer, OsuMatchGet}, osu_mods::OsuModsLazer, BeatmapUserScore, GetUsersResponse, OsuBeatmapAttributes, OsuUser, ScoresBatch};
use reqwest::{
    header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE, COOKIE, USER_AGENT},
    Client, Method, Response, StatusCode,
};

use self::models::{
    osu_leaderboard::OsuLeaderboardLazer, ApiError, GetRanking, GetUserScores,
    OauthResponse, OsuBeatmap, OsuGameMode, OsuScore,
    OsuUserExtended, RankingKind, Rankings, UserId,
};

use std::{fmt::Write, time::Duration};

use self::metrics::Metrics;

use std::sync::Arc;

use tokio::sync::{
    oneshot::{channel, Receiver, Sender},
    RwLock,
};

use crate::{error::OsuApiError, models::OsuUserStatistics};
use serde::de::DeserializeOwned;

static OSU_BASE: &str = "https://osu.ppy.sh";
static OSU_API_BASE: &str = "https://osu.ppy.sh/api/v2";

type ApiResult<T> = Result<T, OsuApiError>;

pub enum ApiKind {
    General,
    Hidden,
    Fallback,
}

#[derive(Debug)]
pub struct OsuApi {
    inner: Arc<OsuToken>,
    fallback_url: String,
    fallback_token: String,
    loop_drop_tx: Option<Sender<()>>,
    osu_session: String,
    pub stats: Metrics,
}

impl Drop for OsuApi {
    fn drop(&mut self) {
        if let Some(tx) = self.loop_drop_tx.take() {
            let _ = tx.send(());
        }
    }
}

#[derive(Debug)]
pub struct OsuToken {
    client: Client,

    client_id: i32,
    secret: String,
    token: RwLock<String>,
}

impl OsuToken {
    async fn request_oauth(&self) -> ApiResult<OauthResponse> {
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
    async fn make_request<T: DeserializeOwned>(
        &self,
        link: &str,
        method: Method,
        api_kind: ApiKind,
        body: Option<String>
    ) -> ApiResult<T> {
        let token = self.inner.token.read().await;

        let r = &self.inner.client;
        let r = match method {
            Method::GET => r.get(link),
            Method::POST => r.post(link),
            _ => unimplemented!(),
        };

        let mut req = match api_kind {
            ApiKind::General => r
                .header(ACCEPT, "application/json")
                .header(CONTENT_TYPE, "application/json")
                .header(AUTHORIZATION, format!("Bearer {token}")),
            ApiKind::Hidden => r
                .header(USER_AGENT, "fumo_potato")
                .header(COOKIE, format!("osu_session={}", self.osu_session)),
            ApiKind::Fallback => r
                .header(USER_AGENT, "fumo_potato")
                .header("x-api-key", &self.fallback_token),
        };

        if let Some(body) = body {
            req = req.body(body)
        }

        let resp = req.send().await?;

        self.handle_error(resp).await
    }

    async fn handle_error<T: DeserializeOwned>(
        &self,
        r: Response,
    ) -> ApiResult<T> {
        let response_url = r.url().as_str().to_owned();
        let response_code = r.status();

        match r.status() {
            StatusCode::OK => {
                // TODO move this nesting mess outta here
                let bytes = r.bytes().await?;
                return serde_json::from_slice::<T>(&bytes).map_err(|s| {
                    // TODO wrap serde error
                    // for more informative response
                    OsuApiError::Parsing {
                        source: s,
                        body: std::str::from_utf8(&bytes).unwrap().to_owned(),
                        url: response_url.clone(),
                    }
                });
            }
            StatusCode::NOT_FOUND => {
                return Err(OsuApiError::NotFound {
                    url: r.url().to_string(),
                })
            }
            StatusCode::TOO_MANY_REQUESTS => {
                return Err(OsuApiError::TooManyRequests)
            }
            StatusCode::UNAUTHORIZED => return Err(OsuApiError::Unauthorized),
            StatusCode::FORBIDDEN => return Err(OsuApiError::Forbidden),
            StatusCode::UNPROCESSABLE_ENTITY => {
                let bytes = r.bytes().await?;

                return Err(OsuApiError::UnprocessableEntity {
                    body: std::str::from_utf8(&bytes).unwrap().to_owned(),
                });
            }
            _ => (),
        };

        let bytes = r.bytes().await?;
        let parsed: ApiError = match serde_json::from_slice(&bytes) {
            Ok(v) => v,
            Err(e) => {
                if bytes.len() <= 1 {
                    return Err(OsuApiError::EmptyBody {
                        code: response_code,
                    });
                }

                return Err(OsuApiError::Parsing {
                    source: e,
                    body: std::str::from_utf8(&bytes).unwrap().to_owned(),
                    url: response_url.clone(),
                });
            }
        };

        Err(OsuApiError::ApiError(parsed))
    }

    pub async fn get_user_scores(
        &self,
        user_scores: GetUserScores,
    ) -> ApiResult<Vec<OsuScore>> {
        let mut link = format!(
            "{OSU_API_BASE}/users/{}/scores/{}?",
            user_scores.user_id, user_scores.kind
        );

        // TODO think of better ways of handling query arguments
        // TODO handle pagination

        if let Some(limit) = user_scores.limit {
            link.push_str(&format!("limit={limit}&"))
        }

        if let Some(fails) = user_scores.include_fails {
            let fails = match fails {
                true => "1",
                false => "0",
            };

            link.push_str(&format!("include_fails={fails}&"))
        }

        if let Some(mode) = user_scores.mode {
            link.push_str(&format!("mode={}&", mode))
        }

        let r = self
            .make_request(
                &link[..link.len() - 1],
                Method::GET,
                ApiKind::General,
                None
            )
            .await?;

        self.stats.counters.with_label_values(&["get_user_scores"]).inc();

        Ok(r)
    }

    pub async fn get_user(
        &self,
        user_id: UserId,
        mode: Option<OsuGameMode>,
    ) -> ApiResult<Option<OsuUserExtended>> {
        let mut link = OSU_API_BASE.to_owned();

        // TODO ?key=
        link.push_str(&format!("/users/{user_id}"));

        if let Some(mode) = mode {
            link.push_str(&format!("/{mode}"))
        }

        let r: ApiResult<OsuUserExtended> = self
            .make_request(&link, Method::GET, ApiKind::General, None)
            .await;

        self.stats.counters.with_label_values(&["get_user"]).inc();

        match r {
            Ok(v) => Ok(Some(v)),
            Err(e) => match e {
                OsuApiError::NotFound { .. } => Ok(None),
                _ => Err(e),
            },
        }
    }

    pub async fn get_user_beatmap_scores(
        &self,
        beatmap_id: i64,
        user_id: UserId,
    ) -> ApiResult<BeatmapUserScore> {
        let link = format!(
            "{OSU_API_BASE}/beatmaps/{}/scores/users/{}",
            beatmap_id, user_id
        );

        let r = self
            .make_request(&link, Method::GET, ApiKind::General, None)
            .await?;

        self.stats.counters.with_label_values(&["get_user_beatmap_scores"]).inc();

        Ok(r)
    }

    pub async fn get_beatmap(&self, bid: i32) -> ApiResult<OsuBeatmap> {
        let link = format!("{OSU_API_BASE}/beatmaps/{bid}");

        let r = self
            .make_request(&link, Method::GET, ApiKind::General, None)
            .await?;
        
        self.stats.counters.with_label_values(&["get_beatmap"]).inc();

        Ok(r)
    }

    pub async fn get_beatmap_attributes(
        &self, 
        bid: i32, 
        mods: Option<&OsuModsLazer>
    ) -> ApiResult<OsuBeatmapAttributes> {
        let mut link = format!("{OSU_API_BASE}/beatmaps/{bid}/attributes");

        if let Some(mods) = mods {
            for (i, osu_mod) in mods.mods.iter().enumerate() {
                if i == 0 {
                    let _ = write!(link, "?mods[]={}", osu_mod.acronym);
                    continue;
                }

                let _ = write!(link, "&mods[]={}", osu_mod.acronym);
            }
        }

        let r = self
            .make_request(&link, Method::POST, ApiKind::General, None)
            .await?;

        self.stats.counters.with_label_values(&["get_beatmap_attributes"]).inc();

        Ok(r)
    }

    pub async fn get_rankings(
        &self,
        ranking: &GetRanking,
        amount: usize,
    ) -> ApiResult<Rankings> {
        let mut link = String::with_capacity(50);
        let mut buffer: Vec<OsuUserStatistics> = Vec::with_capacity(amount);

        let pages: usize = ((amount as f32 / 50.0).ceil() as usize).max(1);

        let pages_offset = ranking.page.unwrap_or(0) as usize;

        let pages_range = if pages_offset != 0 {
            pages_offset..=pages_offset + pages
        } else {
            1..=pages
        };

        for page in pages_range {
            link.clear();
            let _ = write!(
                link,
                "{OSU_API_BASE}/rankings/{}/{}?filter={}&cursor[page]={}",
                ranking.mode, ranking.kind, ranking.filter, page
            );

            if let (RankingKind::Performance, Some(country)) =
                (&ranking.kind, &ranking.country)
            {
                let _ = write!(link, "&country={}", &country);
            };

            let res: Rankings = self
                .make_request(&link, Method::GET, ApiKind::General, None)
                .await?;

            self.stats.counters.with_label_values(&["get_rankings"]).inc();

            let amount_to_append =
                (amount - buffer.len()).min(res.ranking.len());

            buffer.extend_from_slice(&res.ranking[0..amount_to_append])
        }

        Ok(Rankings { ranking: buffer })
    }

    pub async fn get_match_all_events(
        &self,
        match_id: i64,
    ) -> ApiResult<OsuMatchGet> {
        let mut initial: OsuMatchGet =
            self.get_match(match_id, None, None, None).await?;

        let mut last_id = initial.latest_event_id;
        loop {
            let res =
                self.get_match(match_id, None, Some(last_id), None).await?;

            if res.events.is_empty() || last_id == res.latest_event_id {
                break;
            }

            initial.events.extend_from_slice(&res.events);

            last_id = res.latest_event_id;
        }

        Ok(initial)
    }

    pub async fn get_match(
        &self,
        match_id: i64,
        before: Option<i64>,
        after: Option<i64>,
        limit: Option<u8>,
    ) -> ApiResult<OsuMatchGet> {
        let mut link = format!("{OSU_API_BASE}/matches/{}", match_id);

        if let Some(before) = before {
            let _ = write!(link, "&before={}", before);
        }

        if let Some(after) = after {
            let _ = write!(link, "?after={}", after);
        }

        if let Some(limit) = limit {
            let _ = write!(link, "&limit={}", limit);
        }


        self.stats.counters.with_label_values(&["get_match"]).inc();

        self.make_request(&link, Method::GET, ApiKind::General, None)
            .await
    }

    pub async fn get_users(
        &self,
        user_ids: &[i64]
    ) -> ApiResult<GetUsersResponse> {
        let link = format!("{OSU_API_BASE}/users");
        let mut result = Vec::with_capacity(user_ids.len());

        for chunk in user_ids.chunks(50) {
            let mut link = link.clone();

            for (i, id) in chunk.iter().enumerate() {
                if i == 0 {
                    let _ = write!(link, "?ids[]={}", id);
                } else {
                    let _ = write!(link, "&ids[]={}", id);
                }
            }

            self.stats.counters.with_label_values(&["get_users"]).inc();
            let users_response: GetUsersResponse = 
                self.make_request(&link, Method::GET, ApiKind::General, None).await?;
            
            // TODO
            users_response.users.iter().for_each(|v| {
                result.push(v.clone())
            });

            tokio::time::sleep(Duration::from_millis(500)).await;
        }

        Ok(GetUsersResponse {
            users: result
        })
    }

    pub async fn get_scores_batch(
        &self,
        cursor_string: &Option<i64>,
    ) -> ApiResult<ScoresBatch> {
        let mut link = format!("{OSU_API_BASE}/scores");

        if let Some(cursor) = cursor_string {
            let _ = write!(link, "?cursor[id]={}", cursor);
        }

        self.stats.counters.with_label_values(&["get_scores_batch"]).inc();

        self.make_request(
            &link,
            Method::GET,
            ApiKind::General,
            None
        ).await
    }

    pub async fn get_matches_batch(
        &self,
        cursor_string: &Option<i64>,
    ) -> ApiResult<OsuMatchContainer> {
        let mut link = format!("{OSU_API_BASE}/matches");

        if let Some(cursor) = cursor_string {
            let _ = write!(link, "?cursor[id]={}", cursor);
        };

        self.stats.counters.with_label_values(&["get_matches_batch"]).inc();

        self.make_request(
            &link,
            Method::GET,
            ApiKind::General,
            None
        ).await
    }

    pub async fn get_leaderboard_hidden(
        &self,
        bid: i32,
        country: bool,
    ) -> ApiResult<OsuLeaderboardLazer> {
        let mut link = format!("{OSU_BASE}/beatmaps/{bid}/scores?");

        if country {
            link.push_str("type=country")
        }

        self.stats.counters.with_label_values(&["get_leaderboard_hidden"]).inc();
        self.make_request(&link, Method::GET, ApiKind::Hidden, None).await
    }

    // This method works only if FALLBACK_API variable
    // is set.
    pub async fn get_countryleaderboard_fallback(
        &self,
        bid: i32,
        mods: Option<String>
    ) -> ApiResult<FallbackBeatmapScores> {
        let mut link = format!(
            "{}/osu/beatmaps/v2/{}/scores?country=BY&type=country",
            self.fallback_url, bid
        );

        if let Some(mods) = mods {
            let _ = write!(link, "&mods={}", mods);
        }

        // FIXME Temporary solution since seneaL's api is BS
        // at the moment
        let mut retries = 0;
        while retries <= 5 {
            let resp = self
                .make_request(
                    &link, 
                    Method::GET, 
                    ApiKind::Fallback, 
                    None
                )
                .await;


            self.stats.counters.with_label_values(&["get_countryleaderboard"]).inc();

            match resp {
                Ok(r) => return Ok(r),
                Err(e) => {
                    if let OsuApiError::EmptyBody { .. } = e {
                        retries += 1;
                        continue;
                    } else {
                        return Err(e);
                    }
                }
            }
        }

        Err(OsuApiError::ExceededMaxRetries)
    }
}

impl OsuApi {
    pub async fn new(
        client_id: i32,
        secret: &str,
        osu_session: &str,
        fallback_url: &str,
        fallback_token: &str,
        run_loop: bool,
    ) -> ApiResult<OsuApi> {
        let inner = Arc::new(OsuToken {
            client: Client::builder()
                .timeout(Duration::from_secs(2))
                .connect_timeout(Duration::from_secs(2))
                .build()?,
            client_id,
            secret: secret.to_owned(),
            token: Default::default(),
        });

        let response = inner.request_oauth().await?;

        let mut token = inner.token.write().await;
        *token = response.access_token;
        drop(token);

        let (tx, rx) = channel::<()>();

        if run_loop {
            OsuApi::update_token(
                Arc::clone(&inner),
                response.expires_in as u64,
                rx,
            )
            .await;
        }

        let stats = Metrics::new();

        let api = OsuApi {
            loop_drop_tx: Some(tx),
            inner,
            fallback_url: fallback_url.to_owned(),
            osu_session: osu_session.to_owned(),
            stats,
            fallback_token: fallback_token.to_string(),
        };

        Ok(api)
    }

    async fn update_token(osu: Arc<OsuToken>, expire: u64, rx: Receiver<()>) {
        tokio::spawn(async move {
            OsuApi::token_loop(Arc::clone(&osu), expire, rx).await;
            tracing::info!("osu!api token loop is closed!");
        });
    }

    async fn token_loop(
        osu: Arc<OsuToken>,
        mut expire: u64,
        mut rx: Receiver<()>,
    ) {
        loop {
            expire /= 2;
            tracing::info!("osu! token update scheduled in {expire} seconds");
            tokio::select! {
                _ = tokio::time::sleep(Duration::from_secs(expire)) => {}
                _ = &mut rx => {
                    return;
                }
            }

            let response = osu.request_oauth().await.unwrap();

            let mut token = osu.token.write().await;
            *token = response.access_token;

            expire = response.expires_in as u64;
            tracing::info!("Successfully updated osu! token!");
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{models::*, *};
    use std::{
        str::FromStr,
        sync::atomic::{AtomicBool, Ordering::SeqCst},
    };

    use dotenv::dotenv;
    use once_cell::sync::OnceCell;
    use osu_matches::OsuMatchEventKind;
    use std::env;
    use tokio::sync::{Mutex, MutexGuard};

    pub struct OsuApiCell {
        initialized: AtomicBool,
        inner: OnceCell<Mutex<OsuApi>>,
    }

    impl OsuApiCell {
        const fn new() -> Self {
            Self {
                initialized: AtomicBool::new(false),
                inner: OnceCell::new(),
            }
        }

        async fn get(&self) -> Result<MutexGuard<'_, OsuApi>, OsuApiError> {
            let cmp_res = self
                .initialized
                .compare_exchange(false, true, SeqCst, SeqCst);

            if cmp_res.is_ok() {
                dotenv().unwrap();

                let api = OsuApi::new(
                    env::var("CLIENT_ID").unwrap().parse().unwrap(),
                    env::var("CLIENT_SECRET").unwrap().as_str(),
                    env::var("OSU_SESSION").unwrap().as_str(),
                    env::var("FALLBACK_API").unwrap().as_str(),
                    env::var("FALLBACK_API_KEY").unwrap().as_str(),
                    false,
                )
                .await
                .unwrap();

                dbg!(&api);

                self.inner.set(Mutex::new(api)).ok();
            }

            Ok(self.inner.wait().lock().await)
        }
    }

    static API_INSTANCE: OsuApiCell = OsuApiCell::new();

    #[tokio::test]
    async fn test_get_leaderboard_hidden() {
        let api = API_INSTANCE.get().await.unwrap();

        let leaderboard =
            api.get_leaderboard_hidden(1804553, false).await.unwrap();

        assert!(leaderboard.scores.len() == 50);

        let leaderboard =
            api.get_leaderboard_hidden(1804553, true).await.unwrap();

        assert!(leaderboard.scores.len() > 2);
    }

    #[tokio::test]
    async fn test_get_scores() {
        let api = API_INSTANCE.get().await.unwrap();

        let req = GetUserScores::new(6892711, ScoresType::Best).limit(10);

        let scores = api.get_user_scores(req).await.unwrap();
        assert_eq!(scores.len(), 10);

        let req = GetUserScores::new(7562902, ScoresType::Recent)
            .limit(50)
            .include_fails(true);

        api.get_user_scores(req).await.unwrap();
    }

    #[tokio::test]
    async fn test_top_scores_correct_mode() {
        let api = API_INSTANCE.get().await.unwrap();

        let mode = OsuGameMode::Osu;


        let req = GetUserScores {
            user_id: 9211305,
            kind: ScoresType::Best,
            include_fails: Some(false),
            mode: Some(mode),
            limit: Some(100),
            offset: None,
        };

        let scores = api.get_user_scores(req).await.unwrap();

        for score in scores {
            assert_eq!(score.mode, format!("{}", mode))
        }
    }

    #[tokio::test]
    async fn test_get_scores_deser() {
        let api = API_INSTANCE.get().await.unwrap();


        let req = GetUserScores::new(11692602, ScoresType::Best)
            .limit(100)
            .include_fails(true);


        let _scores = api.get_user_scores(req).await.unwrap();
    }

    #[tokio::test]
    async fn test_get_user() {
        let api = API_INSTANCE.get().await.unwrap();

        let user = api
            .get_user(UserId::Id(6892711), None)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(user.id, 6892711);
        assert_eq!(user.username, "LoPij");
        assert_eq!(user.country_code, "BY");

        let user = api
            .get_user(UserId::Username("DaHuJka".to_owned()), None)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(user.id, 6830745);
        assert_eq!(user.username, "DaHuJka");
        assert_eq!(user.country_code, "RU");

        let user = api.get_user(UserId::Id(34785329384), None).await.unwrap();

        assert!(user.is_none());
    }

    #[tokio::test]
    async fn test_get_beatmap() {
        let api = API_INSTANCE.get().await.unwrap();

        let mut op = api.get_beatmap(3153603).await;

        dbg!(&op);

        assert!(op.is_ok());

        let b = op.unwrap();
        assert_eq!(b.id, 3153603);

        op = api.get_beatmap(12).await;
        assert!(op.is_err());

        let op = api.get_beatmap(1173889).await.unwrap();
        assert_eq!(op.status, RankStatus::Loved);

        let op = api.get_beatmap(3833489).await.unwrap();
        assert_eq!(op.status, RankStatus::Graveyard);

        let op = api.get_beatmap(3818011).await.unwrap();
        assert_eq!(op.status, RankStatus::Ranked);
    }

    #[tokio::test]
    async fn test_get_fallback_beatmap_leaderboard() {
        let api = API_INSTANCE.get().await.unwrap();

        let res = api.get_countryleaderboard_fallback(1627148, None).await.unwrap();

        assert_eq!(res.items.len(), 50);
    }

    #[tokio::test]
    async fn test_get_match() {
        let api = API_INSTANCE.get().await.unwrap();


        // Multiple matches to make sure it's deserialize correctly
        let _ = api.get_match(116432947, None, None, None).await.unwrap();
        let _ = api.get_match(116366892, None, None, None).await.unwrap();
        let res = api.get_match(111451190, None, None, None).await.unwrap();

        assert_eq!(res.osu_match.name, "OWC2023: (Canada) VS (Germany)");
        assert_eq!(res.osu_match.id, 111451190);
        assert_eq!(
            res.events.last().unwrap().detail.kind,
            OsuMatchEventKind::MatchDisbanded
        );
    }

    #[tokio::test]
    async fn test_get_match_all_events() {
        let api = API_INSTANCE.get().await.unwrap();

        let res = api.get_match_all_events(111555364).await.unwrap();

        assert_eq!(
            res.osu_match.name,
            "OWC2023: (United States) VS (South Korea)"
        );
        assert_eq!(res.osu_match.id, 111555364);
        assert_eq!(
            res.events.last().unwrap().detail.kind,
            OsuMatchEventKind::MatchDisbanded
        );
        // assert!(res.events.len() > 100); // TODO find a mp find a lot of events

        let res = api.get_match_all_events(116854723).await.unwrap();
        dbg!(res);
    }

    #[tokio::test]
    #[should_panic]
    async fn test_notfound_error() {
        let api = API_INSTANCE.get().await.unwrap();

        let link = "https://osu.ppy.sh/apii/v2/beaaps/";
        let _: OsuBeatmap = api
            .make_request(link, Method::GET, ApiKind::General, None)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_get_beatmap_attributes() {
        let api = API_INSTANCE.get().await.unwrap();

        let res1 = api.get_beatmap_attributes(2025942, None).await.unwrap();

        //assert!(matches!(res1.attributes, OsuBeatmapAttributesKind::Osu{ .. }));

        let mods_dt = OsuModsLazer::from_str("DTHD").unwrap();
        let res2 = api.get_beatmap_attributes(2025942, Some(&mods_dt)).await.unwrap();
        
        /*
        match (res1.attributes, res2.attributes) {
            (OsuBeatmapAttributesKind::Osu { star_rating: nm_sr, .. }, 
             OsuBeatmapAttributesKind::Osu { star_rating: dt_sr, .. }) => {
                assert!(nm_sr != dt_sr);
                assert!(dt_sr > nm_sr);
            }
            (_, _) => panic!("Got non Osu gamemode"),
        }
        */

        // Edge cases
        let res1 = api.get_beatmap_attributes(4878596, None).await.unwrap();
    }

    #[tokio::test]
    async fn test_get_rankings_country() {
        let api = API_INSTANCE.get().await.unwrap();

        let req = GetRanking {
            mode: OsuGameMode::Osu,
            kind: RankingKind::Performance,
            filter: RankingFilter::All,
            country: Some("BY".to_owned()),
            page: None,
        };

        let res = api.get_rankings(&req, 50).await.unwrap();

        assert_eq!(50, res.ranking.len());
        assert_eq!("BY", res.ranking[0].user.country_code);
        assert_eq!("BY", res.ranking[20].user.country_code);
        assert_eq!("BY", res.ranking[49].user.country_code);
    }

    #[tokio::test]
    async fn get_users_batch() {
        let api = API_INSTANCE.get().await.unwrap();

        let res = api.get_users(&[6892711]).await.unwrap();

        assert_eq!(res.users.len(), 1);
        assert_eq!(&res.users[0].username, "LoPij");

        let res = api.get_users(&[6892711, 17851835, 7979597]).await.unwrap();
        assert_eq!(res.users.len(), 3);

        dbg!(res);
    }

    #[tokio::test]
    async fn get_scores_batch() {
        let api = API_INSTANCE.get().await.unwrap();

        let res = api.get_scores_batch(&None).await.unwrap();

        assert!(res.scores.len() != 0)
    }

    #[tokio::test]
    async fn get_matches_batch() {
        let api = API_INSTANCE.get().await.unwrap();

        let res = api.get_matches_batch(&None).await.unwrap();

        assert!(res.matches.len() != 0)
    }

    #[tokio::test]
    async fn test_get_rankings() {
        let api = API_INSTANCE.get().await.unwrap();

        let req = GetRanking {
            mode: OsuGameMode::Osu,
            kind: RankingKind::Performance,
            filter: RankingFilter::All,
            country: None,
            page: None,
        };

        let res = api.get_rankings(&req, 50).await.unwrap();

        assert_eq!(50, res.ranking.len());

        let res = api.get_rankings(&req, 10).await.unwrap();

        assert_eq!(10, res.ranking.len());

        let res = api.get_rankings(&req, 1).await.unwrap();

        assert_eq!(1, res.ranking.len());

        let res = api.get_rankings(&req, 253).await.unwrap();

        assert_eq!(253, res.ranking.len());

        let res = api.get_rankings(&req, 111).await.unwrap();

        assert_eq!(111, res.ranking.len());
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

    #[test]
    fn test_mods_from_str() {
        // Contains one non existent mod
        // Should output DT | EZ
        let mods = OsuMods::from_str("dtdhez").unwrap();
        assert_eq!(mods, OsuMods::DOUBLETIME | OsuMods::EASY);

        let mods = OsuMods::from_str("DTHD").unwrap();
        assert_eq!(mods, OsuMods::DOUBLETIME | OsuMods::HIDDEN);

        let mods = OsuMods::from_str("DTHD").unwrap();
        assert_eq!(mods, OsuMods::DOUBLETIME | OsuMods::HIDDEN);

        let mods = OsuMods::from_str("DThDhR").unwrap();
        assert_eq!(
            mods,
            OsuMods::DOUBLETIME | OsuMods::HIDDEN | OsuMods::HARDROCK
        );

        let mods = OsuMods::from_str("DThDhRdt").unwrap();
        assert_eq!(
            mods,
            OsuMods::DOUBLETIME | OsuMods::HIDDEN | OsuMods::HARDROCK
        );

        let mods = OsuMods::from_str("DTMR").unwrap();
        assert_eq!(mods, OsuMods::DOUBLETIME | OsuMods::MIRROR);
    }

}
