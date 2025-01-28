use lambda_http::{run, service_fn, tracing, Error, Request, Response};

mod http_handler;

const STRAVA_TOKEN_EXCHANGE: &str = "https://www.strava.com";

// TODO: Consider making http_handler know nothing about Strava (specifically it knows client-id
// and client-secret names)
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
