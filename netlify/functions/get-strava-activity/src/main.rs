use aws_lambda_events::encodings::Body;
use aws_lambda_events::event::apigw::{ApiGatewayProxyRequest, ApiGatewayProxyResponse};
use http::header::HeaderMap;
use lambda_runtime::{service_fn, Error, LambdaEvent};
use log::LevelFilter;
use simple_logger::SimpleLogger;

#[tokio::main]
async fn main() -> Result<(), Error> {
    SimpleLogger::new()
        .with_utc_timestamps()
        .with_level(LevelFilter::Info)
        .init()
        .unwrap();

    let func = service_fn(my_handler);
    lambda_runtime::run(func).await?;
    Ok(())
}

pub(crate) async fn my_handler(
    event: LambdaEvent<ApiGatewayProxyRequest>,
) -> Result<ApiGatewayProxyResponse, Error> {
    let token = &event.payload.headers["Authorization"];

    // Here we get the token
    // We validate it with JSONWebToken (see old stuff)
    //// I can't remember if we call jkws.js to get the data off here or if we do that next
    // We call the management API for Auth0 and get user info
    //// Search it to get the access-token for Strava
    // Call Strava and finally send response back to client
    //// We could calculate the drift here but fkuk it let's just use the client side stuff you
    //// already wrote
    let resp = ApiGatewayProxyResponse {
        status_code: 200,
        headers: HeaderMap::new(),
        multi_value_headers: HeaderMap::new(),
        body: Some(Body::Text(format!("Hello from '{:#?}'", token))),
        is_base64_encoded: false,
    };

    Ok(resp)
}
