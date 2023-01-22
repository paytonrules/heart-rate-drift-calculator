use http::response::Builder;
use reqwest::Response;

#[derive(Clone, Copy)]
struct Url<'a>(&'a str);
struct AuthToken<'a>(&'a str);

struct Client<T: SimpleHttpClient> {
    http_client: T,
}

// TODO: Proper error handling
impl<T: SimpleHttpClient> Client<T> {
    pub async fn request(&self, url: Url<'_>, token: AuthToken<'_>) -> Result<Response, Error> {
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

#[derive(Copy, Clone)]
struct NullClient;

impl SimpleHttpClient for NullClient {
    fn new() -> Self {
        Self
    }

    fn get<U: reqwest::IntoUrl>(&self, _url: U) -> Self {
        *self
    }

    fn header<K, V>(self, _key: K, _value: V) -> Self
    where
        http::header::HeaderName: TryFrom<K>,
        <http::header::HeaderName as TryFrom<K>>::Error: Into<http::Error>,
        http::header::HeaderValue: TryFrom<V>,
        <http::header::HeaderValue as TryFrom<V>>::Error: Into<http::Error>,
    {
        self
    }

    async fn send(self) -> Result<Response, reqwest::Error> {
        let response = Builder::new().status(200).body("").unwrap();

        async { Ok(reqwest::Response::from(response)) }.await
    }
}

impl Client<NullClient> {
    pub fn create_null() -> Client<NullClient> {
        Client {
            http_client: NullClient::new(),
        }
    }

    pub fn map_url<T>(self, url: Url, response: http::Response<T>) -> Self {
        self
    }
}

impl NullClient {}

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

        let response = client.request(Url(&url), AuthToken("irrelevant")).await;

        assert!(response.is_ok());

        let json = response?.json::<EchoResponse>().await?;
        assert_eq!(json.auth_header, String::from("Bearer irrelevant"));
        assert_eq!(json.uri, String::from("/request_path"));

        Ok(())
    }

    #[tokio::test]
    async fn null_http_client_returns_empty_response_by_default() -> anyhow::Result<()> {
        let client = Client::create_null();

        let response = client
            .request(Url("http://www.example.com"), AuthToken("token"))
            .await;

        assert_eq!(response?.status(), 200);

        Ok(())
    }

    #[tokio::test]
    async fn null_client_can_map_url_to_response_body() -> anyhow::Result<()> {
        let body = http::Response::builder()
            .status(200)
            .body(hyper::body::Body::from("oh no"))?;

        let client = Client::create_null().map_url(Url("http://example.com/test"), body);

        Ok(())
    }
}
