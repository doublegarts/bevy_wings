//! [`HttpRemote`]: a [`RemoteSource`] for REST resources.
//!
//! `HttpRemote::new("https://api.example.com/notes")` maps the remote operations to:
//!
//! | Operation | Request |
//! | --- | --- |
//! | `fetch(id)` | `GET {base}/{id}` (404 is "not there") |
//! | `fetch_all()` | `GET {base}` returning a JSON array |
//! | `push(entity)` | `PUT {base}/{id}` with the JSON entity |
//! | `remove(id)` | `DELETE {base}/{id}` |
//!
//! Calls are blocking; run them through an [`OfflineFirstRepository`](crate::OfflineFirstRepository)
//! and `commands.entity(e).load(...)`, which executes on Bevy's async compute pool. Status codes
//! become [`DomainError`]s (5xx and network errors are retryable [`DomainError::Unavailable`]).
//!
//! The transport is a [`WingsHttpClient`]: `UreqClient` (feature `http`, TLS with rustls) or
//! your own (an authenticated client, a test fake).

use std::marker::PhantomData;
use std::sync::Arc;

use serde::Serialize;
use serde::de::DeserializeOwned;

use bevy_wings_domain::{DomainEntity, DomainError, DomainResult, Id};

use crate::codec::{Codec, JsonCodec};
use crate::remote::RemoteSource;

/// HTTP method.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WingsHttpMethod {
    /// GET.
    Get,
    /// POST.
    Post,
    /// PUT.
    Put,
    /// DELETE.
    Delete,
}

/// An HTTP request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WingsHttpRequest {
    /// Method.
    pub method: WingsHttpMethod,
    /// Absolute URL.
    pub url: String,
    /// Headers.
    pub headers: Vec<(String, String)>,
    /// Body.
    pub body: Option<Vec<u8>>,
}

/// An HTTP response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WingsHttpResponse {
    /// Status code.
    pub status: u16,
    /// Body.
    pub body: Vec<u8>,
}

/// Sends HTTP requests. Errors are transport failures (no response at all).
pub trait WingsHttpClient: Send + Sync + 'static {
    /// Sends a request and waits for the response.
    fn send(&self, request: WingsHttpRequest) -> Result<WingsHttpResponse, String>;
}

impl<F> WingsHttpClient for F
where
    F: Fn(WingsHttpRequest) -> Result<WingsHttpResponse, String> + Send + Sync + 'static,
{
    fn send(&self, request: WingsHttpRequest) -> Result<WingsHttpResponse, String> {
        self(request)
    }
}

/// HTTP client built on `ureq` with rustls (feature `http`).
#[cfg(feature = "http")]
pub struct UreqClient {
    agent: ureq::Agent,
}

#[cfg(feature = "http")]
impl UreqClient {
    /// A client with a timeout for whole requests.
    pub fn new(timeout: std::time::Duration) -> Self {
        let agent = ureq::Agent::config_builder()
            .timeout_global(Some(timeout))
            .http_status_as_error(false)
            .build()
            .into();
        Self { agent }
    }
}

#[cfg(feature = "http")]
impl Default for UreqClient {
    fn default() -> Self {
        Self::new(std::time::Duration::from_secs(30))
    }
}

#[cfg(feature = "http")]
impl WingsHttpClient for UreqClient {
    fn send(&self, request: WingsHttpRequest) -> Result<WingsHttpResponse, String> {
        let result = match request.method {
            WingsHttpMethod::Get | WingsHttpMethod::Delete => {
                let mut builder = if request.method == WingsHttpMethod::Get {
                    self.agent.get(&request.url)
                } else {
                    self.agent.delete(&request.url)
                };
                for (k, v) in &request.headers {
                    builder = builder.header(k, v);
                }
                builder.call()
            }
            WingsHttpMethod::Post | WingsHttpMethod::Put => {
                let mut builder = if request.method == WingsHttpMethod::Post {
                    self.agent.post(&request.url)
                } else {
                    self.agent.put(&request.url)
                };
                for (k, v) in &request.headers {
                    builder = builder.header(k, v);
                }
                builder.send(request.body.as_deref().unwrap_or_default())
            }
        };
        let mut response = result.map_err(|e| e.to_string())?;
        let status = response.status().as_u16();
        let body = response
            .body_mut()
            .read_to_vec()
            .map_err(|e| e.to_string())?;
        Ok(WingsHttpResponse { status, body })
    }
}

/// Maps an HTTP status to a domain result.
pub fn status_error(status: u16, body: &[u8]) -> Option<DomainError> {
    let text = || {
        String::from_utf8_lossy(body)
            .chars()
            .take(200)
            .collect::<String>()
    };
    match status {
        200..=299 => None,
        401 | 403 => Some(DomainError::Unauthorized),
        404 => Some(DomainError::not_found("resource")),
        409 | 412 => Some(DomainError::Conflict(text())),
        408 | 429 | 500..=599 => Some(DomainError::Unavailable(format!("HTTP {status}"))),
        _ => Some(DomainError::Unexpected(format!(
            "HTTP {status}: {}",
            text()
        ))),
    }
}

/// A REST resource as a [`RemoteSource`].
pub struct HttpRemote<T> {
    base: String,
    client: Arc<dyn WingsHttpClient>,
    headers: Vec<(String, String)>,
    codec: JsonCodec,
    _marker: PhantomData<fn() -> T>,
}

impl<T> HttpRemote<T> {
    /// A resource at `base` (no trailing slash needed) using `client`.
    pub fn with_client(base: impl Into<String>, client: impl WingsHttpClient) -> Self {
        Self {
            base: base.into().trim_end_matches('/').to_string(),
            client: Arc::new(client),
            headers: vec![("Accept".into(), "application/json".into())],
            codec: JsonCodec::default(),
            _marker: PhantomData,
        }
    }

    /// A resource at `base` with the default `ureq` client (feature `http`).
    #[cfg(feature = "http")]
    pub fn new(base: impl Into<String>) -> Self {
        Self::with_client(base, UreqClient::default())
    }

    /// Adds a header to every request (authorization, API keys).
    pub fn with_header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.push((name.into(), value.into()));
        self
    }

    fn request(
        &self,
        method: WingsHttpMethod,
        url: String,
        body: Option<Vec<u8>>,
    ) -> DomainResult<WingsHttpResponse> {
        let mut headers = self.headers.clone();
        if body.is_some() {
            headers.push(("Content-Type".into(), "application/json".into()));
        }
        self.client
            .send(WingsHttpRequest {
                method,
                url,
                headers,
                body,
            })
            .map_err(DomainError::Unavailable)
    }
}

impl<T> RemoteSource<T> for HttpRemote<T>
where
    T: DomainEntity + Serialize + DeserializeOwned,
{
    fn fetch(&self, id: Id<T>) -> DomainResult<Option<T>> {
        let r = self.request(WingsHttpMethod::Get, format!("{}/{}", self.base, id), None)?;
        if r.status == 404 {
            return Ok(None);
        }
        if let Some(e) = status_error(r.status, &r.body) {
            return Err(e);
        }
        Codec::<T>::decode(&self.codec, &r.body).map(Some)
    }

    fn fetch_all(&self) -> DomainResult<Vec<T>> {
        let r = self.request(WingsHttpMethod::Get, self.base.clone(), None)?;
        if let Some(e) = status_error(r.status, &r.body) {
            return Err(e);
        }
        Codec::<Vec<T>>::decode(&self.codec, &r.body)
    }

    fn push(&self, entity: T) -> DomainResult<()> {
        let body = Codec::<T>::encode(&self.codec, &entity)?;
        let r = self.request(
            WingsHttpMethod::Put,
            format!("{}/{}", self.base, entity.id()),
            Some(body),
        )?;
        status_error(r.status, &r.body).map_or(Ok(()), Err)
    }

    fn remove(&self, id: Id<T>) -> DomainResult<()> {
        let r = self.request(
            WingsHttpMethod::Delete,
            format!("{}/{}", self.base, id),
            None,
        )?;
        match r.status {
            404 => Ok(()),
            s => status_error(s, &r.body).map_or(Ok(()), Err),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use std::sync::Mutex;

    #[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
    struct Note {
        id: Id<Note>,
        title: String,
    }

    impl DomainEntity for Note {
        fn id(&self) -> Id<Note> {
            self.id
        }
    }

    /// A tiny in memory REST server.
    fn server() -> (Arc<Mutex<BTreeMap<u64, String>>>, impl WingsHttpClient) {
        let store: Arc<Mutex<BTreeMap<u64, String>>> = Arc::new(Mutex::new(BTreeMap::new()));
        let s = store.clone();
        let client = move |req: WingsHttpRequest| -> Result<WingsHttpResponse, String> {
            let path = req.url.trim_start_matches("https://api.test/notes");
            let id = path.trim_start_matches('/').parse::<u64>().ok();
            let mut map = s.lock().unwrap();
            let ok = |body: String| {
                Ok(WingsHttpResponse {
                    status: 200,
                    body: body.into_bytes(),
                })
            };
            match (req.method, id) {
                (WingsHttpMethod::Get, None) => ok(format!(
                    "[{}]",
                    map.values().cloned().collect::<Vec<_>>().join(",")
                )),
                (WingsHttpMethod::Get, Some(id)) => match map.get(&id) {
                    Some(v) => ok(v.clone()),
                    None => Ok(WingsHttpResponse {
                        status: 404,
                        body: vec![],
                    }),
                },
                (WingsHttpMethod::Put, Some(id)) => {
                    map.insert(id, String::from_utf8(req.body.unwrap()).unwrap());
                    ok(String::new())
                }
                (WingsHttpMethod::Delete, Some(id)) => {
                    map.remove(&id);
                    ok(String::new())
                }
                _ => Ok(WingsHttpResponse {
                    status: 400,
                    body: b"bad".to_vec(),
                }),
            }
        };
        (store, client)
    }

    /// CRUD through the REST mapping.
    #[test]
    fn rest_round_trip() {
        let (_, client) = server();
        let remote = HttpRemote::<Note>::with_client("https://api.test/notes/", client);
        remote
            .push(Note {
                id: Id::new(1),
                title: "milk".into(),
            })
            .unwrap();
        remote
            .push(Note {
                id: Id::new(2),
                title: "ship".into(),
            })
            .unwrap();
        assert_eq!(remote.fetch(Id::new(2)).unwrap().unwrap().title, "ship");
        assert_eq!(remote.fetch(Id::new(9)).unwrap(), None);
        assert_eq!(remote.fetch_all().unwrap().len(), 2);
        remote.remove(Id::new(1)).unwrap();
        assert_eq!(remote.fetch_all().unwrap().len(), 1);
    }

    /// Status codes and transport failures become domain errors.
    #[test]
    fn errors() {
        assert_eq!(
            status_error(503, b""),
            Some(DomainError::Unavailable("HTTP 503".into()))
        );
        assert_eq!(status_error(401, b""), Some(DomainError::Unauthorized));
        assert!(status_error(204, b"").is_none());
        let down = HttpRemote::<Note>::with_client("https://x", |_: WingsHttpRequest| {
            Err("refused".to_string())
        });
        let err = down.fetch_all().unwrap_err();
        assert!(err.is_retryable());
    }
}
