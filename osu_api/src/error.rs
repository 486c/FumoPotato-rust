use reqwest::StatusCode;
use serde::Deserialize;
use thiserror::Error;

use super::models::ApiError;

#[derive(Error, Debug)]
pub enum OsuApiError {
    #[error("failed to parse to str")]
    FromStrError,
    #[error(transparent)]
    ReqwestError(#[from] reqwest::Error),
    #[error("unhandled status code: `{code}` at `{url}`")]
    UnhandledStatusCode { code: u16, url: String },
    #[error("osu! api error: `{0}`")]
    ApiError(#[from] ApiError),
    #[error("not found: `{url}`")]
    NotFound { url: String },
    #[error("serde parsing: body: `{body}`; `{source}` url: {url}")]
    Parsing {
        source: serde_json::Error,
        body: String,
        url: String,
    },
    #[error("too many requests")]
    TooManyRequests,
    #[error("unprocessable entity: `{body}`")]
    UnprocessableEntity { body: String },
    #[error("service unavailable")]
    ServiceUnavailable,
    #[error("empty body: code: `{code}`")]
    EmptyBody { code: StatusCode },
    #[error("exceeded max retries")]
    ExceededMaxRetries,
    #[error("forbidden")]
    Forbidden,
    #[error("unthorized")]
    Unauthorized,
    #[error("error serialazing")]
    Serializing(#[from] serde_json::Error),
    #[error("casting error")]
    Casting,
    #[error("cursor is too old")]
    CursorTooOld,
}
