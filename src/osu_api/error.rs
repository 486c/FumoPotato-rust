use super::models::ApiError;
use std::{error::Error as StdError, fmt};

#[derive(Debug)]
pub enum OsuApiError {
    FromStrError,
    ReqwestError { err: reqwest::Error },
    UnhandledStatusCode { code: u16, url: String },
    ApiError { source: ApiError },
    NotFound { url: String },
    Parsing { source: serde_json::Error, body: hyper::body::Bytes },
    TooManyRequests,
    UnprocessableEntity,
    ServiceUnavailable,
    EmptyBody,
    ExceededMaxRetries,
}

impl From<reqwest::Error> for OsuApiError {
    fn from(value: reqwest::Error) -> Self {
        OsuApiError::ReqwestError {
            err: value
        }
    }
}

impl StdError for OsuApiError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            OsuApiError::FromStrError => None,
            OsuApiError::ReqwestError { err } => Some(err),
            OsuApiError::UnhandledStatusCode { .. } => None,
            OsuApiError::ApiError { source } => Some(source),
            OsuApiError::NotFound { .. } => None,
            OsuApiError::Parsing { .. } => None,
            OsuApiError::TooManyRequests => None,
            OsuApiError::UnprocessableEntity => None,
            OsuApiError::ServiceUnavailable => None,
            OsuApiError::EmptyBody => None,
            OsuApiError::ExceededMaxRetries => None,
        }
    }
}

impl fmt::Display for OsuApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OsuApiError::FromStrError => 
                f.write_str("converting to string error"),
            OsuApiError::ReqwestError { .. } => 
                f.write_str("Got reqwest error!"),
            OsuApiError::UnhandledStatusCode { .. } => 
                f.write_str("Got unknown status code"),
            OsuApiError::ApiError { .. } => 
                f.write_str("Got internal osu!api error"),
            OsuApiError::NotFound { .. } => 
                f.write_str("Url doesn't found"),
            OsuApiError::Parsing { .. } => 
                f.write_str("Got error during json parsing"),
            OsuApiError::TooManyRequests => f.write_str("Got 429!"),
            OsuApiError::UnprocessableEntity => 
                f.write_str("Got unprocessable entity"),
            OsuApiError::ServiceUnavailable => 
                f.write_str("Service is unavailable"),
            OsuApiError::EmptyBody => 
                f.write_str("Got empty response"),
            OsuApiError::ExceededMaxRetries => 
                f.write_str("Exceeded max retries for api call"),
        }
    }
}

