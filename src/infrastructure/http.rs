use http::response::Builder;
use reqwest::Response;

struct Url(&'static str);
struct AuthToken(&'static str);

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
    pub async fn request(&self, url: Url, token: AuthToken) -> Result<Response, Error> {
        Err(Error::Unknown)
    }
}

// Meant to be used as http::error
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
    use actix_web::{get, web, App, HttpRequest, HttpServer, Responder, Result};
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
        body: String,
    }

    #[get("/{path}")]
    async fn index(path: web::Path<String>, req: HttpRequest) -> Result<impl Responder> {
        println!("I get here the world is good!");
        let body = path.into_inner();
        let auth_header = String::from(
            req.headers()
                .get("Authorization")
                .unwrap()
                .to_str()
                .unwrap(),
        );
        let obj = EchoResponse { auth_header, body };
        Ok(web::Json(obj))
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn focused_integration_test_on_client() {
        // Start echo server server in one thread
        let server = tokio::spawn({
            HttpServer::new(|| App::new().service(index))
                .bind("127.0.0.1:8081")
                .expect("Could not bind test server to port 8081")
                .run()
        });

        // create real client
        let client = Client::new();

        // Make request
        // TODO Reminder you aren't actually making the request yet, you need to
        // test drive this in
        // await might actually let the Http server run. Not sure
        let response = client
            .request(
                Url("https://127.0.0.1:8081/request_path"),
                AuthToken("Bearer irrelevant"),
            )
            .await;

        assert!(response.is_ok());
        let json = response.unwrap().json::<EchoResponse>().await;
        assert!(json.is_ok());
        let response = json.unwrap();

        assert_eq!(response.auth_header, String::from("Bearer irrelevant"));
        assert_eq!(response.body, String::from("request_path"));
        // How to kill the http server? Will exiting the test take care of it?
    }

    // Then the rest is unit tests with headers,
    // then you work back up to the application layer, which is kind of a mess right now (I think mod.rs has a bunch of
    // duplicate code
}
