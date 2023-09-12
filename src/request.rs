use std::{collections::HashMap, fmt};

use http::{HeaderName, HeaderValue, Method};
use serde::de::DeserializeOwned;
use url::Url;

/// An incoming request to an instance of [`MockServer`].
///
/// Each matcher gets an immutable reference to a `Request` instance in the [`matches`] method
/// defined in the [`Match`] trait.
///
/// [`MockServer`]: crate::MockServer
/// [`matches`]: crate::Match::matches
/// [`Match`]: crate::Match
///
/// ### Implementation notes:
/// We can't use `http_types::Request` directly in our `Match::matches` signature:
/// it requires having mutable access to the request to extract the body (which gets
/// consumed when read!).
/// It would also require `matches` to be async, which is cumbersome due to the lack of async traits.
///
/// We introduce our `Request` type to perform this extraction once when the request
/// arrives in the mock serve, store the result and pass an immutable reference to it
/// to all our matchers.
#[derive(Debug, Clone)]
pub struct Request {
    pub url: Url,
    pub method: Method,
    pub headers: HashMap<HeaderName, Vec<HeaderValue>>,
    pub body: Vec<u8>,
}

impl fmt::Display for Request {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "{} {}", self.method, self.url)?;
        for (name, values) in &self.headers {
            let values = values
                .iter()
                .map(|value| String::from_utf8(value.as_bytes().to_owned()).unwrap())
                .collect::<Vec<_>>();
            let values = values.join(",");
            writeln!(f, "{}: {}", name, values)?;
        }
        writeln!(f, "{}", String::from_utf8_lossy(&self.body))
    }
}

impl Request {
    pub fn body_json<T: DeserializeOwned>(&self) -> Result<T, serde_json::Error> {
        serde_json::from_slice(&self.body)
    }

    pub async fn from(request: http::Request<hyper::Body>) -> Request {
        Self::from_hyper(request).await
    }

    pub(crate) async fn from_hyper(request: hyper::Request<hyper::Body>) -> Request {
        let (parts, body) = request.into_parts();
        let method = parts.method;
        let url = match parts.uri.authority() {
            Some(_) => parts.uri.to_string(),
            None => format!("http://localhost{}", parts.uri),
        }
        .parse()
        .unwrap();

        let mut headers = HashMap::new();
        for (name, value) in parts.headers.iter() {
            headers
                .entry(name.clone())
                .and_modify(|values: &mut Vec<HeaderValue>| {
                    values.push(value.clone());
                })
                .or_insert_with(|| vec![value.clone()]);
        }

        let body = hyper::body::to_bytes(body)
            .await
            .expect("Failed to read request body.")
            .to_vec();

        Self {
            url,
            method,
            headers,
            body,
        }
    }
}
