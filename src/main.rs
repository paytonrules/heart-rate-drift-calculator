mod heart_rate_drift;
use heart_rate_drift::HeartRateDriftError;

use actix_web::{web, App, HttpRequest, HttpServer, Responder};
use oauth2::{basic::BasicClient, AuthUrl, ClientId, ClientSecret, CsrfToken, RedirectUrl, Scope};
use serde::Deserialize;

const OAUTH_URL: &str = "https://www.strava.com/oauth/authorize";
const CLIENT_ID: &str = "96911";
const CLIENT_SECRET: &str = "4def338c11c2d0ba69eee13bdf9761f7bd6fe090";
const REDIRECT_URI: &str = "http://localhost:8000";
const SCOPE_READ: &str = "read,activity:read";

#[derive(Deserialize)]
struct AuthToken {
    code: String,
}

async fn authenticate(req: web::Query<AuthToken>) -> impl Responder {
    format!("Authorization request for code={}!", req.code)
}

#[tokio::main]
async fn main() -> Result<(), HeartRateDriftError> {
    // Start Server
    let redirect_server = HttpServer::new(|| App::new().route("/", web::get().to(authenticate)))
        .bind("127.0.0.1:8000")
        .map_err(|_err| HeartRateDriftError::NotEnoughSamples)?
        .run();

    // Make Request
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

    if webbrowser::open(auth_url.as_str()).is_ok() {
        println!("Wait for the server");
        redirect_server
            .await
            .map_err(|_err| HeartRateDriftError::NotEnoughSamples)?;
        println!("Done waiting");
    }

    Ok(())

    // Start server
    /*
    // Get code and store it ....somewhere on the redirect ....Arc<RefCell>
    // Shutdown server

    // Make API call to it to get the info from the given race (via command line)
    // https://www.strava.com/api/v3/activities/7944016770/streams?keys=heartrate,time&key_by_type=true

    // Calculate HR drift and spit it out
    // combine_hr_with_time().heart_rate_drift();*/
}
