use lambda_http::{run, service_fn, tracing, Error};
mod http_handler;
use http_handler::parse_redirect_from_strava;

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing::init_default_subscriber();

    run(service_fn(parse_redirect_from_strava)).await
}
