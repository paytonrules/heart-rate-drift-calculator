use core::future::Future;
use http::response::Builder;
use reqwest::Response;

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

struct Client {
    reqwest_client: reqwest::Client,
    reqwest_builder: Option<reqwest::RequestBuilder>,
}

// Note - the client can explode! In order to simplify the interface (specifically
// so I wouldn't have to mimic all the builders in reqwest with APIs and nullables)
// I unified this into one API.
// However because client has the reqwest_builder, and it can be None, this can crash
// Don't make it public
impl SimpleHttpClient for Client {
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
        self.clone()
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
}
