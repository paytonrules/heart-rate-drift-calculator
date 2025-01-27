use lambda_http::{Error, Request, RequestExt, Response};
use std::env;

const CLIENT_ID_KEY: &str = "STRAVA_CLIENT";
const CLIENT_SECRET_KEY: &str = "STRAVA_CLIENT_SECRET";

/// The secret service trait allows injecting secrets via a templated function
/// By default it uses the enviornment
pub(crate) trait SecretService {
    fn get(&self, key: &str) -> anyhow::Result<String> {
        env::var(key).map_err(anyhow::Error::from)
    }
}

pub(crate) struct EnvironmentSecretService {}
impl SecretService for EnvironmentSecretService {}

/// This is the main body for the function.
/// Write your code inside it.
/// There are some code example in the following URLs:
/// - https://github.com/awslabs/aws-lambda-rust-runtime/tree/main/examples
pub(crate) async fn parse_redirect_from_strava<T: SecretService>(
    event: Request,
    url: &str,
    secret_service: &T,
) -> Result<Response<String>, Error> {
    // Get the code from the event
    // TODO: unwrap_or may not be ideal
    let code = event
        .query_string_parameters_ref()
        .and_then(|params| params.first("code"))
        .unwrap_or("");

    let client = reqwest::Client::new();
    let path = format!("{}{}", url, "/oauth/token");

    // TODO: unwrap_or_default will work but confusing.
    let token_exchange = client
        .post(path)
        .form(&[
            ("code", code),
            (
                "client_id",
                &secret_service.get(CLIENT_ID_KEY).unwrap_or_default(),
            ),
            (
                "client_secret",
                &secret_service.get(CLIENT_SECRET_KEY).unwrap_or_default(),
            ),
        ])
        .send()
        .await
        .unwrap();

    // Return something that implements IntoResponse.
    // It will be serialized to the right response event automatically by the runtime
    let resp = Response::builder()
        .status(200)
        .header("content-type", "text/html")
        .body(token_exchange.text().await.unwrap_or_default())
        .map_err(Box::new)?;

    Ok(resp)
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Context;
    use lambda_http::{Request, RequestExt};
    use std::collections::HashMap;
    use wiremock::matchers::{body_string_contains, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[derive(Default)]
    struct TestSecretService {
        lookup: std::collections::HashMap<String, String>,
    }

    impl TestSecretService {
        fn add(&mut self, key: &str, value: &str) {
            self.lookup.insert(key.to_owned(), value.to_owned());
        }
    }

    impl SecretService for TestSecretService {
        fn get(&self, key: &str) -> anyhow::Result<String> {
            self.lookup.get(key).context("key is not present").cloned()
        }
    }

    // TODO: Return result so you can use ? syntax
    #[tokio::test]
    async fn test_proper_redirect_with_code_returns_access_token_from_post() {
        // We have a fairly large setup here.
        // First, setup an 'secret service' with values for the client id and client secret
        const TEST_CLIENT_ID: &str = "Test_Strava_Client_ID";
        const TEST_CLIENT_SECRET: &str = "Test_Strava_Client_Secret";

        let mut test_secret_service = TestSecretService::default();
        test_secret_service.add(CLIENT_ID_KEY, TEST_CLIENT_ID);
        test_secret_service.add(CLIENT_SECRET_KEY, TEST_CLIENT_SECRET);

        // Now we prepare the incoming request that is sent by an OAuth redirect url
        // The key is that it will have the query string code=<code>
        const OAUTH_CODE: &str = "12345";

        let query_string: HashMap<String, String> =
            HashMap::from([(String::from("code"), OAUTH_CODE.to_string())]);
        let request = Request::default().with_query_string_parameters(query_string);

        // Finally prepare a mock server expecting the oauth/token request with the code
        // from the original request and the client id and secret key
        const THE_TEST_TOKEN: &str = "The Test Token";
        let mock_server = MockServer::start().await;
        // TODO: URI should be passed in
        // TODO: Make hard coded strings here (uri, query params) constants
        Mock::given(method("POST"))
            .and(path("/oauth/token"))
            .and(body_string_contains(format!("code={}", OAUTH_CODE)))
            .and(body_string_contains(format!(
                "client_id={}",
                TEST_CLIENT_ID
            )))
            .and(body_string_contains(format!(
                "client_secret={}",
                TEST_CLIENT_SECRET
            )))
            .respond_with(ResponseTemplate::new(200).set_body_string(THE_TEST_TOKEN))
            .mount(&mock_server)
            .await;

        // Act - call our redirect parser
        let actual_response =
            parse_redirect_from_strava(request, &mock_server.uri(), &test_secret_service)
                .await
                .unwrap();

        let body_string = actual_response.body();
        let requests = mock_server.received_requests().await;
        println!("The requests {:#?}", requests);
        assert_eq!(body_string, THE_TEST_TOKEN);
    }
}
