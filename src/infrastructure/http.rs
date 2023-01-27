use std::collections::HashMap;

use http::response::Builder;
use reqwest::Response;

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
struct Url(String);
#[derive(PartialEq, Eq, Hash, Clone, Debug)]
struct AuthToken(String);

struct Client<T: SimpleHttpClient> {
    http_client: T,
}

// TODO: Proper error handling
impl<T: SimpleHttpClient> Client<T> {
    // TODO: Maybe this should not return a reqwest Response object, although
    // if you're only supposed to use this through Strava it should be okay
    pub async fn request(&self, url: Url, token: AuthToken) -> Result<Response, Error> {
        self.http_client
            .get(url.0)
            .header("Authorization", format!("Bearer {}", token.0))
            .send()
            .await
            .map_err(|_err| {
                println!("err {} {_err}", _err.is_connect());
                Error::Unknown
            })
    }
}

impl Client<ReqwestWrapper> {
    pub fn new() -> Self {
        Client {
            http_client: ReqwestWrapper::new(),
        }
    }
}

// Meant to be used as http::Error
// Should probably have the response error in it
#[derive(thiserror::Error, PartialEq, Debug)]
pub enum Error {
    #[error("Unknown error")]
    Unknown,
}

trait SimpleHttpClient {
    fn new() -> Self;
    fn get<U: reqwest::IntoUrl>(&self, url: U) -> Self;
    fn header<K, V>(self, key: K, value: V) -> Self
    where
        http::header::HeaderName: TryFrom<K>,
        <http::header::HeaderName as TryFrom<K>>::Error: Into<http::Error>,
        http::header::HeaderValue: TryFrom<V>,
        <http::header::HeaderValue as TryFrom<V>>::Error: Into<http::Error>;
    async fn send(self) -> Result<Response, reqwest::Error>;
}

struct ReqwestWrapper {
    reqwest_client: reqwest::Client,
    reqwest_builder: Option<reqwest::RequestBuilder>,
}

// Note - the client can explode! In order to simplify the interface (specifically
// so I wouldn't have to mimic all the builders in reqwest with APIs and nullables)
// I unified this into one API. However because client has the reqwest_builder, and
// it can be None, this can crash.
// Don't make it public
impl SimpleHttpClient for ReqwestWrapper {
    fn new() -> Self {
        Self {
            reqwest_client: reqwest::Client::new(),
            reqwest_builder: None,
        }
    }

    fn get<U: reqwest::IntoUrl>(&self, url: U) -> Self {
        Self {
            reqwest_client: self.reqwest_client.clone(),
            reqwest_builder: Some(self.reqwest_client.get(url)),
        }
    }

    fn header<K, V>(self, key: K, value: V) -> Self
    where
        http::header::HeaderName: TryFrom<K>,
        <http::header::HeaderName as TryFrom<K>>::Error: Into<http::Error>,
        http::header::HeaderValue: TryFrom<V>,
        <http::header::HeaderValue as TryFrom<V>>::Error: Into<http::Error>,
    {
        Self {
            reqwest_client: self.reqwest_client,
            reqwest_builder: Some(
                self.reqwest_builder
                    .expect("No request builder present")
                    .header(key, value),
            ),
        }
    }

    async fn send(self) -> Result<Response, reqwest::Error> {
        self.reqwest_builder
            .expect("Error - no request builder present")
            .send()
            .await
    }
}

#[derive(Clone)]
struct NullClient {
    url: Option<String>,
    auth_token: Option<String>,
    request_map: HashMap<Option<AuthToken>, HashMap<Url, String>>,
}

impl NullClient {
    pub fn map_url(mut self, url: Url, response: String) -> Self {
        let mut url_map = self
            .request_map
            .get(&None)
            .unwrap_or(&HashMap::default())
            .clone();
        url_map.insert(url, response);
        self.request_map.insert(None, url_map);
        self
    }

    pub fn map_authenticated_url(mut self, token: AuthToken, url: Url, response: String) -> Self {
        let mut url_map = self
            .request_map
            .get(&Some(token.clone()))
            .unwrap_or(&HashMap::default())
            .clone();
        url_map.insert(url, response);
        self.request_map.insert(Some(token), url_map);
        self
    }
}

impl SimpleHttpClient for NullClient {
    fn new() -> Self {
        Self {
            url: None,
            auth_token: None,
            request_map: HashMap::new(),
        }
    }

    fn get<U: reqwest::IntoUrl>(&self, url: U) -> Self {
        Self {
            url: Some(url.as_str().to_string()),
            auth_token: self.auth_token.clone(),
            request_map: self.request_map.clone(),
        }
    }

    fn header<K, V>(self, key: K, value: V) -> Self
    where
        http::header::HeaderName: TryFrom<K>,
        <http::header::HeaderName as TryFrom<K>>::Error: Into<http::Error>,
        http::header::HeaderValue: TryFrom<V>,
        <http::header::HeaderValue as TryFrom<V>>::Error: Into<http::Error>,
    {
        match <http::header::HeaderName as TryFrom<K>>::try_from(key) {
            Ok(header) => {
                let inner = header.as_str();

                if inner == "authorization" {
                    match <http::header::HeaderValue as TryFrom<V>>::try_from(value) {
                        Ok(value) => {
                            let token_string = value
                                .to_str()
                                .map(|s| s.strip_prefix("Bearer ").unwrap_or_default())
                                .map(|s| s.to_string())
                                .ok();

                            Self {
                                request_map: self.request_map,
                                auth_token: token_string,
                                url: self.url,
                            }
                        }
                        Err(_) => self,
                    }
                } else {
                    self
                }
            }
            Err(_) => self,
        }
    }

    async fn send(self) -> Result<Response, reqwest::Error> {
        let token = self.auth_token.map(AuthToken);
        let url_map = self
            .request_map
            .get(&token)
            .or(self.request_map.get(&None))
            .unwrap_or(&HashMap::default())
            .clone();

        let response_body = self
            .url
            .and_then(|url_string| url_map.get(&Url(url_string)).cloned());

        let response = if let Some(response_body) = response_body {
            Builder::new().status(200).body(response_body).unwrap()
        } else {
            Builder::new().status(401).body("".to_string()).unwrap()
        };

        async { Ok(reqwest::Response::from(response)) }.await
    }
}

impl Client<NullClient> {
    pub fn create_null() -> Client<NullClient> {
        Client {
            http_client: NullClient::new(),
        }
    }

    pub fn map_url(self, url: Url, response: String) -> Self {
        Self {
            http_client: self.http_client.map_url(url, response),
        }
    }

    pub fn map_authenticated_url(self, token: AuthToken, url: Url, response: String) -> Self {
        Self {
            http_client: self.http_client.map_authenticated_url(token, url, response),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    mod server;
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize)]
    struct EchoResponse {
        auth_header: String,
        uri: String,
    }

    fn start_echo_server() -> server::Server {
        server::http(|req| async move {
            let uri = req.uri().to_string();
            let auth_header = String::from(
                req.headers()
                    .get("Authorization")
                    .and_then(|val| val.to_str().ok())
                    .unwrap_or_default(),
            );

            let response = serde_json::to_string(&EchoResponse { auth_header, uri })
                .expect("Could not serialize echo");

            http::Response::builder()
                .status(200)
                .body(hyper::body::Body::from(response))
                .unwrap()
        })
    }

    #[tokio::test]
    async fn focused_integration_test_for_client() -> anyhow::Result<()> {
        let server = start_echo_server();
        let client = Client::new();
        let url = format!("http://{}/request_path", server.addr());

        let response = client
            .request(Url(url.into()), AuthToken("irrelevant".to_owned()))
            .await;

        assert!(response.is_ok());

        let json = response?.json::<EchoResponse>().await?;
        assert_eq!(json.auth_header, String::from("Bearer irrelevant"));
        assert_eq!(json.uri, String::from("/request_path"));

        Ok(())
    }

    #[tokio::test]
    async fn null_http_client_returns_unauthenticated_response_by_default() -> anyhow::Result<()> {
        let client = Client::create_null();

        let response = client
            .request(
                Url("http://www.example.com".to_owned()),
                AuthToken("token".to_owned()),
            )
            .await;

        assert_eq!(response?.status(), 401);

        Ok(())
    }

    #[tokio::test]
    async fn null_client_can_map_unauthenticated_url_to_response_body() -> anyhow::Result<()> {
        let client = Client::create_null().map_url(
            Url("http://example.com/test".to_owned()),
            "stored response".to_owned(),
        );

        let response = client
            .request(
                Url("http://example.com/test".to_owned()),
                AuthToken("doesnt matter".to_owned()),
            )
            .await?;

        assert_eq!(response.text().await?, "stored response");

        Ok(())
    }

    #[tokio::test]
    async fn null_client_can_create_authenticated_urls() -> anyhow::Result<()> {
        let token = AuthToken("token".to_owned());
        let url = Url("http://example.com/test".to_owned());
        let client = Client::create_null().map_authenticated_url(
            token.clone(),
            url.clone(),
            "stored response".to_owned(),
        );

        let response = client.request(url.clone(), token.clone()).await?;
        assert_eq!(response.status(), 200);
        assert_eq!(response.text().await?, "stored response");

        let incorrect_token = AuthToken("bad liar".to_owned());
        let response = client.request(url, incorrect_token).await?;

        assert_eq!(response.status(), 401);

        Ok(())
    }

    // What to do with an unmapped URL?
    // Can we map multiple URLS to the same token?
}
