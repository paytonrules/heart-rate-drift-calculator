use std::env;

use anyhow::Result;
use aws_lambda_events::encodings::Body;
use aws_lambda_events::event::apigw::{ApiGatewayProxyRequest, ApiGatewayProxyResponse};
use http::header::HeaderMap;
use lambda_runtime::{service_fn, Error, LambdaEvent};
use lambda_runtime_api_client::BoxError;
use log::LevelFilter;
use serde::{Deserialize, Serialize};
use simple_logger::SimpleLogger;

const CLIENT_ID_KEY: &str = "STRAVA_CLIENT_ID";
const CLIENT_SECRET_KEY: &str = "STRAVA_CLIENT_SECRET";
const STRAVA_TOKEN_EXCHANGE: &str = "https://www.strava.com/oauth/token";

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

#[derive(Serialize, Deserialize, Default, Debug, PartialEq)]
struct StravaTokenResponse {
    access_token: String,
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
        let params = connector_config.params();
        let client = reqwest::Client::new();
        client
            .post(&connector_config.uri)
            .form(&params)
            .send()
            .await
            .map_err(anyhow::Error::from)
    }
}

trait Environment {
    fn get(&self, key: &str) -> anyhow::Result<String> {
        env::var(key).map_err(anyhow::Error::from)
    }
}

struct SystemEnvironment {}
impl Environment for SystemEnvironment {}

async fn redirect_from_strava<T: StravaConnector, U: Environment>(
    event: LambdaEvent<ApiGatewayProxyRequest>,
    strava_connection: &T,
    environment: &U,
) -> std::result::Result<ApiGatewayProxyResponse, Error> {
    let code = event
        .payload
        .query_string_parameters
        .first("code")
        .ok_or("No Code Present")?;

    let strava_response = strava_connection
        .request_token(
            &StravaConnectorConfig::default()
                .uri(STRAVA_TOKEN_EXCHANGE)
                .client_id(&environment.get(CLIENT_ID_KEY).map_err(BoxError::from)?)
                .client_secret(&environment.get(CLIENT_SECRET_KEY).map_err(BoxError::from)?)
                .code(code),
        )
        .await?;

    let status_code = strava_response.status().as_u16() as i64;
    let access_token = if strava_response.status().is_success() {
        let token: StravaTokenResponse = serde_json::from_str(&strava_response.text().await?)?;
        Some(token.access_token)
    } else {
        None
    };

    let resp = ApiGatewayProxyResponse {
        status_code,
        headers: HeaderMap::new(),
        multi_value_headers: HeaderMap::new(),
        body: Some(Body::Text(access_token.unwrap_or_default())),
        is_base64_encoded: false,
    };

    Ok(resp)
}

pub(crate) async fn default_redirect(
    event: LambdaEvent<ApiGatewayProxyRequest>,
) -> Result<ApiGatewayProxyResponse, Error> {
    redirect_from_strava(event, &HttpStravaConnector {}, &SystemEnvironment {}).await
}

#[cfg(test)]
mod tests {
    use anyhow::{anyhow, bail};
    use aws_lambda_events::query_map::QueryMap;
    use http::response::Builder;
    use lambda_runtime::Context;
    use lambda_runtime_api_client::BoxError;
    use std::collections::HashMap;

    use super::*;

    const CLIENT_SECRET: &str = "ClientSecret";
    const CLIENT_ID: &str = "11111";

    struct MockStravaConnector {
        code: u16,
        config: StravaConnectorConfig,
        token_response: StravaTokenResponse,
    }

    impl Default for MockStravaConnector {
        fn default() -> Self {
            Self {
                code: 200,
                config: StravaConnectorConfig::default(),
                token_response: StravaTokenResponse::default(),
            }
        }
    }

    impl MockStravaConnector {
        fn with_expected_config(config: &StravaConnectorConfig) -> Self {
            Self {
                config: config.clone(),
                ..Default::default()
            }
        }

        fn with_error_code(code: u16) -> Self {
            Self {
                code,
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
            // Short circuit - doesn't matter what's in the request if the code is not 200
            // The default code is 200 so anything that is successful must pass the other checks as
            // well
            if self.code != 200 {
                //                let url = Url::parse("http://example.com").unwrap();
                let response = Builder::new()
                    .status(self.code)
                    //                   .url(url.clone())
                    .body("")
                    .unwrap();
                let response = reqwest::Response::from(response);

                return Ok(response);
            }

            // This is a stub. It should only work if the expected config is passed into the
            // request (so one with a matching code, client id and client secret (remember thats
            // the whole reason for a back end!))
            if config.uri != self.config.uri && !self.config.uri.is_empty() {
                bail!(
                    "Strava Uri is missing or incorrect. Passed URI: {}",
                    config.uri
                );
            }

            if config.code != self.config.code && !self.config.code.is_empty() {
                bail!(
                    "Strava Code is missing or incorrect. Passed Code: {}",
                    config.code
                );
            }

            if config.client_id != self.config.client_id && !self.config.client_id.is_empty() {
                bail!(
                    "Strava Client ID is missing or incorrect. Passed Client ID: {}",
                    config.client_id
                );
            }

            if config.client_secret != self.config.client_secret
                && !self.config.client_secret.is_empty()
            {
                bail!(
                    "Strava Client Secret is missing or incorrect. Passed Client Secret: {}",
                    config.client_secret
                );
            }

            let response = reqwest::Response::from(http::Response::new(serde_json::to_string(
                &self.token_response,
            )?));
            Ok(response)
        }
    }

    #[derive(Default)]
    struct MockEnvironment {
        environment_map: std::collections::HashMap<String, String>,
    }

    impl MockEnvironment {
        fn with_client_secrets() -> Self {
            let mut environment = Self::default();
            environment
                .environment_map
                .insert(CLIENT_ID_KEY.to_string(), CLIENT_ID.to_string());
            environment
                .environment_map
                .insert(CLIENT_SECRET_KEY.to_string(), CLIENT_SECRET.to_string());
            environment
        }
    }

    impl Environment for MockEnvironment {
        fn get(&self, key: &str) -> anyhow::Result<String> {
            self.environment_map
                .get(&String::from(key))
                .cloned()
                .ok_or(anyhow!("Ooops"))
        }
    }

    fn expected_base_request_config() -> StravaConnectorConfig {
        StravaConnectorConfig::default()
            .uri(STRAVA_TOKEN_EXCHANGE)
            .client_id(CLIENT_ID)
            .client_secret(CLIENT_SECRET)
    }

    fn create_redirect_event_with_code(code: &str) -> LambdaEvent<ApiGatewayProxyRequest> {
        let mut query_string = HashMap::new();
        query_string.insert("code".into(), vec![code.into()]);
        let query_string_parameters = QueryMap::from(query_string);
        let payload = ApiGatewayProxyRequest {
            query_string_parameters,
            ..Default::default()
        };

        LambdaEvent::new(payload, Context::default())
    }

    #[tokio::test]
    async fn test_proper_redirect_with_code_returns_access_token_from_post() -> Result<(), BoxError>
    {
        const THE_TEST_TOKEN: &str = "The Test Token";
        const RESPONSE_CODE: &str = "12345";
        let event = create_redirect_event_with_code(RESPONSE_CODE);

        // The connector is configured with the expected url, client_id, client_secret
        // and expected RESPONSE_CODE. It will error if the connctor is called incorrectly
        let connector = MockStravaConnector::with_expected_config(
            &expected_base_request_config().code(RESPONSE_CODE),
        )
        .and_token_response(THE_TEST_TOKEN.into());

        // Act - call the redirect
        let actual_response_body =
            &redirect_from_strava(event, &connector, &MockEnvironment::with_client_secrets())
                .await?
                .body
                .ok_or("Body is not present")?;

        // assert!(matches! should work here, or maybe assert_matches!
        match actual_response_body {
            Body::Text(body_text) => assert_eq!(body_text, THE_TEST_TOKEN),
            _ => panic!("The response body of the redirect was not text"),
        };

        Ok(())
    }

    #[tokio::test]
    async fn test_proper_redirect_without_code_is_error() {
        let payload = ApiGatewayProxyRequest::default();
        let context = Context::default();
        let event = LambdaEvent::new(payload, context);

        assert!(redirect_from_strava(
            event,
            &MockStravaConnector::with_expected_config(&expected_base_request_config()),
            &MockEnvironment::with_client_secrets()
        )
        .await
        .is_err());
    }

    #[tokio::test]
    async fn test_non_successful_status_code_from_strava_connector() -> Result<(), BoxError> {
        let connector = MockStravaConnector::with_error_code(500);
        let event = create_redirect_event_with_code("validcode");

        let response =
            redirect_from_strava(event, &connector, &MockEnvironment::with_client_secrets())
                .await?;

        assert_eq!(response.status_code, 500);

        Ok(())
    }

    // TODO: Test what happens when environment variables are missing

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
