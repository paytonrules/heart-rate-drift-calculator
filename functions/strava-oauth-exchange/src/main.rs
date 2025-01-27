use lambda_http::{run, service_fn, tracing, Error, Request, Response};
use tokio;

mod http_handler;

const STRAVA_TOKEN_EXCHANGE: &str = "https://www.strava.com";
// TODO: You'll need to make `parse_redirect_from_strava` into the version with the specific
// strava url and environment config
// Perhaps the name is exchange_for_token_from_oauth_url
// http_handler will have the more generic version
// that's for testability (Environment doesnt need to be changed, URL is injected)
// but does feel more separated

async fn parse_redirect_from_strava(event: Request) -> Result<Response<String>, Error> {
    http_handler::parse_redirect_from_strava(
        event,
        STRAVA_TOKEN_EXCHANGE,
        &http_handler::EnvironmentSecretService {},
    )
    .await
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing::init_default_subscriber();

    run(service_fn(parse_redirect_from_strava)).await
}
