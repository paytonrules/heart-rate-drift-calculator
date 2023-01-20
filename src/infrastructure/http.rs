use http::response::Builder;
use reqwest::Response;

struct Url<'a>(&'a str);
struct AuthToken<'a>(&'a str);

struct Client<T: SimpleHttpClient> {
    http_client: T,
}

impl Client<ReqwestWrapper> {
    pub fn new() -> Self {
        Client {
            http_client: ReqwestWrapper::new(),
        }
    }
}

impl Client<NullClient> {
    pub fn create_null() -> Client<NullClient> {
        Client {
            http_client: NullClient::new(),
        }
    }
}

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

// Meant to be used as http::Error
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
// I unified this into one API.
// However because client has the reqwest_builder, and it can be None, this can crash
// Don't make it public
// TODO Maybe this can just be the reqwest_builder
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

    fn get<U: reqwest::IntoUrl>(&self, url: U) -> Self {
        *self
    }

    fn header<K, V>(self, key: K, value: V) -> Self
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

#[cfg(test)]
mod tests {
    use super::*;
    mod server;
    use actix_web::{get, web, App, HttpRequest, HttpServer, Responder};
    use serde::{Deserialize, Serialize};

    #[tokio::test]
    async fn null_http_client_returns_empty_response() {
        let client = NullClient::new();

        let response = client
            .get("http://ignore")
            .header("irrelevant", "value")
            .send()
            .await;

        assert_eq!(response.unwrap().status(), 200);
    }

    #[derive(Serialize, Deserialize)]
    struct EchoResponse {
        auth_header: String,
        uri: String,
    }
    /*
    #[get("/{path}")]
    async fn index(path: web::Path<String>, req: HttpRequest) -> Result<impl Responder> {
        println!("Into the path");
        let body = path.into_inner();
        let auth_header = String::from(
            req.headers()
                .get("Authorization")
                .and_then(|val| val.to_str().ok())
                .unwrap_or_default(),
        );
        let obj = EchoResponse { auth_header, body };
        Ok(web::Json(obj))
    }*/

    #[tokio::test]
    async fn focused_integration_test_for_client() -> anyhow::Result<()> {
        let server = server::http(|req| async move {
            let uri = req.uri().to_string();
            let auth_header = String::from(
                req.headers()
                    .get("Authorization")
                    .and_then(|val| val.to_str().ok())
                    .unwrap_or_default(),
            );

            let obj = EchoResponse { auth_header, uri };
            let response = serde_json::to_string(&obj).expect("Could not serialize echo");

            http::Response::builder()
                .status(200)
                .body(hyper::body::Body::from(response))
                .unwrap()
        });

        // create real client
        let client = Client::new();

        let url = format!("http://{}/request_path", server.addr());
        // Make request
        let response = client.request(Url(&url), AuthToken("irrelevant")).await;

        assert!(response.is_ok());

        let json = response?.json::<EchoResponse>().await?;
        assert_eq!(json.auth_header, String::from("Bearer irrelevant"));
        assert_eq!(json.uri, String::from("/request_path"));

        Ok(())
    }

    // Then the rest is unit tests with headers,
    // then you work back up to the application layer, which is kind of a mess right now (I think mod.rs has a bunch of
    // duplicate code
}
