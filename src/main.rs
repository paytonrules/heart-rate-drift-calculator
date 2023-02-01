#![feature(async_fn_in_trait)]

mod heart_rate_drift;
mod infrastructure;

use heart_rate_drift::HeartRateDriftError;

use actix_web::{web, App, HttpServer, Responder};
use oauth2::{basic::BasicClient, AuthUrl, ClientId, ClientSecret, CsrfToken, RedirectUrl, Scope};
use serde::Deserialize;

const OAUTH_URL: &str = "https://www.strava.com/oauth/authorize";
const CLIENT_ID: &str = "96911";
const CLIENT_SECRET: &str = "4def338c11c2d0ba69eee13bdf9761f7bd6fe090";
const REDIRECT_URI: &str = "http://localhost:8000";
const SCOPE_READ: &str = "activity:read_all";

#[derive(Deserialize)]
struct AuthToken {
    code: String,
}

#[derive(Deserialize, Debug)]
struct ExchangeResponse {
    access_token: String,
}

async fn authenticate(req: web::Query<AuthToken>) -> impl Responder {
    let client = reqwest::ClientBuilder::new().build().expect("BOOM?");
    let res_exchange = client
        .post("https://www.strava.com/oauth/token")
        .query(&[
            ("client_id", CLIENT_ID),
            ("client_secret", CLIENT_SECRET),
            ("code", &req.code),
            ("grant_type", "authorization_code"),
        ])
        .send()
        .await
        .expect("Mother puss bucket")
        .json::<ExchangeResponse>()
        .await
        .expect("Boom Oauth");

    let res = client
        .get("https://www.strava.com/api/v3/activities/7944016770/streams?keys=heartrate,time&key_by_type=true")
        .header("Authorization", "Bearer ".to_owned() + &res_exchange.access_token)
        .send()
        .await
        .expect("BOOM")
        .text()
        .await
        .expect("BOOM FOR REAL");

    format!("Result equals (warning probably big) {}", res)
}

#[tokio::main]
async fn main() -> Result<(), HeartRateDriftError> {
    // Start Server
    let redirect_server = HttpServer::new(|| App::new().route("/", web::get().to(authenticate)))
        .bind("127.0.0.1:8000")
        .map_err(|_err| HeartRateDriftError::NotEnoughSamples)?
        .run();

    // Construct OAUTH Request
    let client = BasicClient::new(
        ClientId::new(CLIENT_ID.to_string()),
        Some(ClientSecret::new(CLIENT_SECRET.to_string())),
        AuthUrl::new(OAUTH_URL.to_string())
            .map_err(|_err| HeartRateDriftError::NotEnoughSamples)?,
        None,
    )
    // Set the URL the user will be redirected to after the authorization process.
    .set_redirect_uri(
        RedirectUrl::new(REDIRECT_URI.to_string())
            .map_err(|_err| HeartRateDriftError::NotEnoughSamples)?,
    );

    let (auth_url, _csrf_token) = client
        .authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new(SCOPE_READ.to_string()))
        .url();
    println!("auth_url {}", auth_url.as_str());

    if webbrowser::open(auth_url.as_str()).is_ok() {
        redirect_server
            .await
            .map_err(|_err| HeartRateDriftError::NotEnoughSamples)?;
    }

    Ok(())
}

struct Application {}

impl Application {
    fn new() -> Self {
        Application {}
    }

    fn drift(&self) -> String {
        String::from("0.0%")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initial_app_test() {
        let app = Application::new();

        assert_eq!(String::from("0.0%"), app.drift());
    }
}
