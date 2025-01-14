use anyhow::{anyhow, bail, Result};
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

    let func = service_fn(default_redirect);
    lambda_runtime::run(func).await?;
    Ok(())
}

trait StravaConnector {}

struct HttpStravaConnector {}

impl StravaConnector for HttpStravaConnector {}

pub(crate) async fn redirect_from_strava<T: StravaConnector>(
    event: LambdaEvent<ApiGatewayProxyRequest>,
    strava_connection: T,
) -> Result<ApiGatewayProxyResponse, Error> {
    let code = event
        .payload
        .query_string_parameters
        .first("code")
        .ok_or("No Code Present")?;

    // We have the code and need to exchange it with the access token
    //
    let resp = ApiGatewayProxyResponse {
        status_code: 200,
        headers: HeaderMap::new(),
        multi_value_headers: HeaderMap::new(),
        body: Some(Body::Text(format!("Hello from Me"))),
        is_base64_encoded: false,
    };

    Ok(resp)
}

pub(crate) async fn default_redirect(
    event: LambdaEvent<ApiGatewayProxyRequest>,
) -> Result<ApiGatewayProxyResponse, Error> {
    redirect_from_strava(event, HttpStravaConnector {}).await
}

#[cfg(test)]
mod tests {
    use aws_lambda_events::query_map::QueryMap;
    use lambda_runtime::Context;
    use std::collections::HashMap;

    use super::*;

    struct MockStravaConnector {}

    impl StravaConnector for MockStravaConnector {}

    #[tokio::test]
    async fn test_proper_redirect_with_code_is_ok() {
        let mut query_string = HashMap::new();
        query_string.insert("code".into(), vec!["12345".into()]);
        let query_string_parameters = QueryMap::from(query_string);
        let payload = ApiGatewayProxyRequest {
            query_string_parameters,
            ..Default::default()
        };

        let context = Context::default();
        let event = LambdaEvent::new(payload, context);

        assert!(redirect_from_strava(event, MockStravaConnector {})
            .await
            .is_ok());
    }

    #[tokio::test]
    async fn test_proper_redirect_without_code_is_error() {
        let payload = ApiGatewayProxyRequest::default();
        let context = Context::default();
        let event = LambdaEvent::new(payload, context);

        assert!(redirect_from_strava(event, MockStravaConnector {})
            .await
            .is_err());
    }
}
