//! Common utilities for Cincinnati backend.

#![deny(missing_docs)]

extern crate actix_web;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate serde_derive;

mod config;
pub use crate::config::MergeOptions;

pub mod de;
pub mod metrics;
pub mod testing;
pub mod tracing;

mod errors;
pub use errors::{register_metrics, Fallible, GraphError, MISSING_APPSTATE_PANIC_MSG};

/// Commonly used imports for error handling.
pub mod prelude_errors {
    pub use crate::errors::prelude::*;
}

use actix_web::http::{header, HeaderMap};
use std::collections::HashMap;
use std::collections::HashSet;
use url::form_urlencoded;

lazy_static! {
    /// list of cincinnati versions
    pub static ref CINCINNATI_VERSION: HashMap<&'static str, i32> =
        [("application/vnd.redhat.cincinnati.v1+json", 1)]
            .iter()
            .cloned()
            .collect();
    /// minimum cincinnati version supported
    pub static ref MIN_CINCINNATI_VERSION: &'static str = "application/vnd.redhat.cincinnati.v1+json";
}

/// Strip all but one leading slash and all trailing slashes
pub fn parse_path_prefix<S>(path_prefix: S) -> String
where
    S: AsRef<str>,
{
    format!("/{}", path_prefix.as_ref().to_string().trim_matches('/'))
}

/// Deserialize path_prefix
pub fn de_path_prefix<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::Deserialize;
    let path_prefix = String::deserialize(deserializer)?;
    Ok(Some(parse_path_prefix(path_prefix)))
}

/// Parse a comma-separated set of client parameters keys.
pub fn parse_params_set<S>(params: S) -> HashSet<String>
where
    S: AsRef<str>,
{
    params
        .as_ref()
        .split(',')
        .filter_map(|key| {
            let trimmed = key.trim().to_string();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed)
            }
        })
        .collect()
}

/// Make sure `query` string contains all `params` keys.
pub fn ensure_query_params(
    required_params: &HashSet<String>,
    query: &str,
) -> Result<(), GraphError> {
    // No mandatory parameters, always fine.
    if required_params.is_empty() {
        return Ok(());
    }

    // Extract and de-duplicate keys from input query.
    let query_keys: HashSet<String> = form_urlencoded::parse(query.as_bytes())
        .into_owned()
        .map(|(k, _)| k)
        .collect();

    // Make sure no mandatory parameters are missing.
    let mut missing: Vec<String> = required_params.difference(&query_keys).cloned().collect();
    if !missing.is_empty() {
        missing.sort();
        return Err(GraphError::MissingParams(missing));
    }

    Ok(())
}

/// Make sure the client can accept the provided media type.
pub fn validate_content_type(
    headers: &HeaderMap,
    mut content_type: Vec<actix_web::http::HeaderValue>,
) -> Result<String, GraphError> {
    let header_value = match headers.get(header::ACCEPT) {
        None => {
            let minimum_version = MIN_CINCINNATI_VERSION.to_string();
            return Ok(minimum_version);
        }
        Some(v) => v,
    };

    let wildcard = header::HeaderValue::from_static("*");
    let double_wildcard = header::HeaderValue::from_static("*/*");

    let mut top_types: Vec<actix_web::http::HeaderValue> = content_type
        .iter()
        .map(|ct| {
            let top_type = ct.to_str().unwrap_or("").split("/").next().unwrap_or("");
            let top_type_wildcard = header::HeaderValue::from_str(&format!("{}/*", top_type));
            assert!(
                top_type_wildcard.is_ok(),
                "could not form top-type wildcard from {}",
                top_type
            );
            top_type_wildcard.unwrap()
        })
        .collect();

    let mut acceptable_content_types: Vec<actix_web::http::HeaderValue> =
        vec![wildcard, double_wildcard];
    acceptable_content_types.append(&mut content_type);
    acceptable_content_types.append(&mut top_types);

    // FIXME: this is not a full-blown Accept parser
    if acceptable_content_types.iter().any(|c| c == header_value) {
        let minimum_version: String = MIN_CINCINNATI_VERSION.to_string();
        let accept = header::HeaderValue::to_str(header_value);
        return match accept {
            Ok(a) => Ok(a.parse().unwrap()),
            Err(_e) => Ok(minimum_version),
        };
    } else {
        Err(GraphError::InvalidContentType)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_path_prefix() {
        assert_eq!(parse_path_prefix("//a/b/c//"), "/a/b/c");
        assert_eq!(parse_path_prefix("/a/b/c/"), "/a/b/c");
        assert_eq!(parse_path_prefix("/a/b/c"), "/a/b/c");
        assert_eq!(parse_path_prefix("a/b/c"), "/a/b/c");
    }

    #[test]
    fn test_parse_params_set() {
        assert_eq!(parse_params_set(""), HashSet::new());

        let basic = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        assert_eq!(parse_params_set("a,b,c"), basic.into_iter().collect());

        let dedup = vec!["a".to_string(), "b".to_string()];
        assert_eq!(parse_params_set("a,b,a"), dedup.into_iter().collect());

        let trimmed = vec!["foo".to_string(), "bar".to_string()];
        assert_eq!(
            parse_params_set("foo , , bar"),
            trimmed.into_iter().collect()
        );
    }

    #[test]
    fn test_ensure_query_params() {
        let empty = HashSet::new();
        ensure_query_params(&empty, "").unwrap();
        ensure_query_params(&empty, "a=b").unwrap();

        let simple = vec!["a".to_string()].into_iter().collect();
        ensure_query_params(&simple, "a=b").unwrap();
        ensure_query_params(&simple, "a=b&a=c").unwrap();
        ensure_query_params(&simple, "").unwrap_err();
        ensure_query_params(&simple, "c=d").unwrap_err();
    }

    #[test]
    fn test_validate_content_type() {
        let mut headers = actix_web::http::HeaderMap::new();

        let content_type: Vec<actix_web::http::HeaderValue> =
            vec![header::HeaderValue::from_str("application/json").unwrap()];
        let version = validate_content_type(&headers, content_type.clone()).unwrap(); // if the request leaves Accept empty, we return the minimum supported cincinnati version
        assert_eq!(version, MIN_CINCINNATI_VERSION.to_string());

        headers.insert(
            header::ACCEPT,
            //"application/json, text/*; q=0.2".parse().unwrap(), // prefer JSON, but also accept any text/* after an 80% markdown in quality.  FIXME: needs a smarter parser in validate_content_type
            "application/json".parse().unwrap(),
        );
        let version = validate_content_type(&headers, content_type.clone()).unwrap();
        assert_eq!(version, "application/json");

        headers.insert(
            // FIXME: drop once validate_content_type gets a smarter parser and the previous insert can include the text/* entry
            header::ACCEPT,
            "text/*".parse().unwrap(),
        );
        let text_type: Vec<actix_web::http::HeaderValue> =
            vec![header::HeaderValue::from_str("text/plain").unwrap()];
        let version = validate_content_type(&headers, text_type).unwrap();
        assert_eq!(version, "text/*");

        let image_type: Vec<actix_web::http::HeaderValue> =
            vec![header::HeaderValue::from_str("image/png").unwrap()];
        validate_content_type(&headers, image_type).unwrap_err();

        headers.insert(
            // FIXME: drop once validate_content_type gets a smarter parser and the previous insert can include the text/* entry
            header::ACCEPT,
            "application/vnd.redhat.cincinnati.v1+json".parse().unwrap(),
        );
        let cincinnati_v1: Vec<actix_web::http::HeaderValue> =
            vec![
                header::HeaderValue::from_str("application/vnd.redhat.cincinnati.v1+json").unwrap(),
            ];
        let version = validate_content_type(&headers, cincinnati_v1).unwrap(); // if the request leaves Accept empty, we can return whatever we want
        assert_eq!(version, "application/vnd.redhat.cincinnati.v1+json");
    }
}
