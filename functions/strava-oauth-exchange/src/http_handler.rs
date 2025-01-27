use anyhow::Context;
use lambda_http::{Error, Request, RequestExt, Response};
use std::env;

const OAUTH_TOKEN_PATH: &str = "/oauth/token";
const CODE_QUERY_PARAM_NAME: &str = "code";
const CLIENT_ID_QUERY_PARAM_NAME: &str = "client_id";
const CLIENT_SECRET_QUERY_PARAM_NAME: &str = "client_secret";

// TODO: There is a bit of a mix and match here as you've got 'strava' specific code here,
// but also generic code here. I'm essentially undecided - should everyting strava be in main?
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
    let code = event
        .query_string_parameters_ref()
        .and_then(|params| params.first(CODE_QUERY_PARAM_NAME))
        .context("Code query param is not present")?;

    let client = reqwest::Client::new();
    let path = format!("{}{}", url, OAUTH_TOKEN_PATH);

    let token_exchange = client
        .post(path)
        .form(&[
            (CODE_QUERY_PARAM_NAME, code),
            (
                CLIENT_ID_QUERY_PARAM_NAME,
                // TODO These unwrap_or_default calls are pointless and wrong
                &secret_service.get(CLIENT_ID_KEY)?,
            ),
            (
                CLIENT_SECRET_QUERY_PARAM_NAME,
                &secret_service.get(CLIENT_SECRET_KEY)?,
            ),
        ])
        .send()
        .await? // I don't have a test for this `?` - sue me
        .error_for_status()?;

    // Return something that implements IntoResponse.
    // It will be serialized to the right response event automatically by the runtime
    let resp = Response::builder()
        .status(200)
        .header("content-type", "text/html")
        .body(token_exchange.text().await?)
        .map_err(Box::new)?;

    Ok(resp)
}

#[cfg(test)]
mod tests {
    use super::*;
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

    fn create_test_secret_service(client_id: &str, client_secret: &str) -> TestSecretService {
        // We have a fairly large setup here.
        // First, setup an 'secret service' with values for the client id and client secret
        let mut test_secret_service = TestSecretService::default();
        test_secret_service.add(CLIENT_ID_KEY, client_id);
        test_secret_service.add(CLIENT_SECRET_KEY, client_secret);
        test_secret_service
    }

    fn create_strava_incoming_request_with_code(code: &str) -> Request {
        let query_string: HashMap<String, String> =
            HashMap::from([(CODE_QUERY_PARAM_NAME.to_string(), code.to_string())]);
        Request::default().with_query_string_parameters(query_string)
    }

    #[tokio::test]
    async fn test_proper_redirect_with_code_returns_access_token_from_post(
    ) -> Result<(), Box<Error>> {
        const TEST_CLIENT_ID: &str = "Test_Strava_Client_ID";
        const TEST_CLIENT_SECRET: &str = "Test_Strava_Client_Secret";

        // First, setup an 'secret service' with values for the client id and client secret
        let test_secret_service = create_test_secret_service(TEST_CLIENT_ID, TEST_CLIENT_SECRET);

        // Now we prepare the incoming request that is sent by an OAuth redirect url
        // The key is that it will have the query string code=<code>
        const OAUTH_CODE: &str = "12345";
        let request = create_strava_incoming_request_with_code(OAUTH_CODE);

        // Finally prepare a mock server expecting the oauth/token request with the
        // code, client id and client secret
        // If done right it will return the test token
        const THE_TEST_TOKEN: &str = "The Test Token";
        let mock_server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(OAUTH_TOKEN_PATH))
            .and(body_string_contains(format!(
                "{}={}",
                CODE_QUERY_PARAM_NAME, OAUTH_CODE
            )))
            .and(body_string_contains(format!(
                "{}={}",
                CLIENT_ID_QUERY_PARAM_NAME, TEST_CLIENT_ID
            )))
            .and(body_string_contains(format!(
                "{}={}",
                CLIENT_SECRET_QUERY_PARAM_NAME, TEST_CLIENT_SECRET
            )))
            .respond_with(ResponseTemplate::new(200).set_body_string(THE_TEST_TOKEN))
            .mount(&mock_server)
            .await;

        // Act - call our redirect parser
        let actual_response =
            parse_redirect_from_strava(request, &mock_server.uri(), &test_secret_service).await?;

        let body_string = actual_response.body();
        assert_eq!(body_string, THE_TEST_TOKEN);
        Ok(())
    }

    #[tokio::test]
    async fn test_proper_redirect_without_code_is_error() {
        let test_secret_service = create_test_secret_service("irrelevant", "irrelevant");
        let request_without_code_param = Request::default();

        let actual_response = parse_redirect_from_strava(
            request_without_code_param,
            "http://www.example.com",
            &test_secret_service,
        )
        .await;

        assert!(actual_response.is_err());
    }

    #[tokio::test]
    async fn test_missing_client_id_is_error() {
        let mut test_secret_service = TestSecretService::default();
        test_secret_service.add(CLIENT_SECRET_KEY, "irrelevant");

        let valid_request = create_strava_incoming_request_with_code("1");

        let actual_response = parse_redirect_from_strava(
            valid_request,
            "http://www.example.com",
            &test_secret_service,
        )
        .await;

        assert!(actual_response.is_err());
    }

    #[tokio::test]
    async fn test_missing_environment_client_secret_is_error() {
        let mut test_secret_service = TestSecretService::default();
        test_secret_service.add(CLIENT_ID_KEY, "irrelevant");

        let valid_request = create_strava_incoming_request_with_code("1");

        let actual_response = parse_redirect_from_strava(
            valid_request,
            "http://www.example.com",
            &test_secret_service,
        )
        .await;

        assert!(actual_response.is_err());
    }

    #[tokio::test]
    async fn test_404_returns_error() {
        let test_secret_service = create_test_secret_service("irrelevant", "irrelevant");
        let request = create_strava_incoming_request_with_code("1");

        let mock_server = MockServer::start().await;
        // Note the mock server doesn't have any URLs so everything should be a 404
        let actual_response =
            parse_redirect_from_strava(request, &mock_server.uri(), &test_secret_service).await;

        assert!(actual_response.is_err());
    }
}
