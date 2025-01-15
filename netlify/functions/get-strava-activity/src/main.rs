use anyhow::{anyhow, bail, Result};
use aws_lambda_events::encodings::Body;
use aws_lambda_events::event::apigw::{ApiGatewayProxyRequest, ApiGatewayProxyResponse};
use http::header::HeaderMap;
use lambda_runtime::{service_fn, Error, LambdaEvent};
use log::LevelFilter;
use simple_logger::SimpleLogger;

const CLIENT_ID: &str = "11111";
const CLIENT_SECRET: &str = "SECRET";
const STRAVA_TOKEN: &str = "https://www.strava.com/oauth/token";

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

#[derive(Default, Clone)]
struct StravaConnectorConfig {
    client_id: String,
    client_secret: String,
    code: String,
    uri: String,
}

impl StravaConnectorConfig {
    fn client_id(mut self, client_id: &str) -> StravaConnectorConfig {
        self.client_id = client_id.into();
        self
    }

    fn client_secret(mut self, client_secret: &str) -> StravaConnectorConfig {
        self.client_secret = client_secret.into();
        self
    }

    fn code(mut self, code: &str) -> StravaConnectorConfig {
        self.code = code.into();
        self
    }

    fn uri(mut self, uri: &str) -> StravaConnectorConfig {
        self.uri = uri.into();
        self
    }

    fn params(&self) -> [(&'static str, String); 3] {
        [
            ("client_id", self.client_id.clone()),
            ("client_secret", self.client_secret.clone()),
            ("code", self.code.clone()),
        ]
    }
}

trait StravaConnector {
    async fn request_token(
        &self,
        connector_config: &StravaConnectorConfig,
    ) -> anyhow::Result<reqwest::Response>;
}

struct HttpStravaConnector {}

impl StravaConnector for HttpStravaConnector {
    async fn request_token(
        &self,
        connector_config: &StravaConnectorConfig,
    ) -> anyhow::Result<reqwest::Response> {
        let params = [("", "money")];
        let client = reqwest::Client::new();
        client
            .post("https://www.strava.com/oauth/token")
            .form(&params)
            .send()
            .await
            .map_err(anyhow::Error::from)
    }
}

pub(crate) async fn redirect_from_strava<T: StravaConnector>(
    event: LambdaEvent<ApiGatewayProxyRequest>,
    strava_connection: T,
) -> std::result::Result<ApiGatewayProxyResponse, Error> {
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
    use serde::{Deserialize, Serialize};
    use std::collections::HashMap;

    use super::*;

    const THE_TEST_TOKEN: &str = "The Test Token";

    #[derive(Serialize, Deserialize, Default, Debug, PartialEq)]
    struct StravaTokenResponse {
        access_token: String,
    }

    #[derive(Default)]
    struct MockStravaConnector {
        config: StravaConnectorConfig,
        token_response: StravaTokenResponse,
    }

    impl MockStravaConnector {
        fn with_expected_config(config: &StravaConnectorConfig) -> Self {
            MockStravaConnector {
                config: config.clone(),
                ..Default::default()
            }
        }

        fn and_token_response(mut self, access_token: String) -> Self {
            self.token_response = StravaTokenResponse { access_token };
            self
        }
    }

    impl StravaConnector for MockStravaConnector {
        async fn request_token(
            &self,
            config: &StravaConnectorConfig,
        ) -> anyhow::Result<reqwest::Response> {
            // This is a stub. It should only work if the expected config is passed into the
            // request (so one with a matching code, client id and client secret (remember thats
            // the whole reason for a back end!))
            //
            /*        res: hyper::Response<ResponseBody>,
            url: Url,
            accepts: Accepts,
            total_timeout: Option<Pin<Box<Sleep>>>,
            read_timeout: Option<Duration>,*/
            let response = reqwest::Response::from(http::Response::new(serde_json::to_string(
                &self.token_response,
            )?));
            Ok(response)
        }
    }

    fn expected_base_request_config() -> StravaConnectorConfig {
        StravaConnectorConfig::default()
            .uri(STRAVA_TOKEN)
            .client_id(CLIENT_ID)
            .client_secret(CLIENT_SECRET)
    }

    #[tokio::test]
    async fn test_proper_redirect_with_code_returns_access_token_from_post() {
        let expected_code = "12345";
        let mut query_string = HashMap::new();
        query_string.insert("code".into(), vec![expected_code.into()]);
        let query_string_parameters = QueryMap::from(query_string);
        let payload = ApiGatewayProxyRequest {
            query_string_parameters,
            ..Default::default()
        };

        let context = Context::default();
        let event = LambdaEvent::new(payload, context);

        let connector = MockStravaConnector::with_expected_config(&expected_base_request_config())
            .and_token_response(THE_TEST_TOKEN.into());

        let raw_response_body = redirect_from_strava(event, connector)
            .await
            .unwrap()
            .body
            .unwrap();

        let actual_response_body: StravaTokenResponse =
            serde_json::from_slice(&raw_response_body).unwrap();

        let expected_response_body = StravaTokenResponse {
            access_token: THE_TEST_TOKEN.to_string(),
        };
        assert_eq!(actual_response_body, expected_response_body);
    }

    #[tokio::test]
    async fn test_proper_redirect_without_code_is_error() {
        let payload = ApiGatewayProxyRequest::default();
        let context = Context::default();
        let event = LambdaEvent::new(payload, context);

        assert!(redirect_from_strava(
            event,
            MockStravaConnector::with_expected_config(&expected_base_request_config())
        )
        .await
        .is_err());
    }

    #[test]
    fn test_strava_connector_returns_params_as_reqwest_params() {
        let config = StravaConnectorConfig::default()
            .client_id("ClientID")
            .client_secret("Secret")
            .code("code");

        assert_eq!(
            config.params(),
            [
                ("client_id", "ClientID".to_owned()),
                ("client_secret", "Secret".to_owned()),
                ("code", "code".to_owned())
            ]
        );
    }

    #[test]
    fn test_strava_connector_has_uri_that_is_not_in_params() {
        let config = StravaConnectorConfig::default().uri("http://example.com");

        assert_eq!(
            config.params(),
            [
                ("client_id", "".to_owned()),
                ("client_secret", "".to_owned()),
                ("code", "".to_owned())
            ]
        );
        assert_eq!(config.uri, "http://example.com");
    }
}
