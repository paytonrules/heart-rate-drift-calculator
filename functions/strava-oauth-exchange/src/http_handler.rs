use lambda_http::{Body, Error, Request, RequestExt, Response};

/// This is the main body for the function.
/// Write your code inside it.
/// There are some code example in the following URLs:
/// - https://github.com/awslabs/aws-lambda-rust-runtime/tree/main/examples
pub(crate) async fn parse_redirect_from_strava(event: Request) -> Result<Response<Body>, Error> {
    // Extract some useful information from the request
    let who = event
        .query_string_parameters_ref()
        .and_then(|params| params.first("name"))
        .unwrap_or("world");
    let message = format!("Hello {who}, this is an AWS Lambda HTTP request");

    // Return something that implements IntoResponse.
    // It will be serialized to the right response event automatically by the runtime
    let resp = Response::builder()
        .status(200)
        .header("content-type", "text/html")
        .body(message.into())
        .map_err(Box::new)?;
    Ok(resp)
}

#[cfg(test)]
mod tests {
    use super::*;
    use lambda_http::{Request, RequestExt};
    use std::collections::HashMap;
    use wiremock::MockServer;

    #[tokio::test]
    async fn test_generic_http_handler() {
        let request = Request::default();

        let response = parse_redirect_from_strava(request).await.unwrap();
        assert_eq!(response.status(), 200);

        let body_bytes = response.body().to_vec();
        let body_string = String::from_utf8(body_bytes).unwrap();

        assert_eq!(
            body_string,
            "Hello world, this is an AWS Lambda HTTP request"
        );
    }
    /*
    #[tokio::test]
    async fn test_proper_redirect_with_code_returns_access_token_from_post() -> Result<(), BoxError>
    {
        let mock_server = MockServer::start().await;

        const RESPONSE_CODE: &str = "12345";
        const THE_TEST_TOKEN: &str = "The Test Token";
        let query_string = HashMap::from([("code", RESPONSE_CODE)]);
        let request = Request::default().with_query_string_parameters(query_string.into());

        // Replace the MockStravaConnector with a wiremock expectation

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
    */

    #[tokio::test]
    async fn test_http_handler_with_query_string() {
        let mut query_string_parameters: HashMap<String, String> = HashMap::new();
        query_string_parameters.insert("name".into(), "strava-oauth-exchange".into());

        let request = Request::default().with_query_string_parameters(query_string_parameters);

        let response = parse_redirect_from_strava(request).await.unwrap();
        assert_eq!(response.status(), 200);

        let body_bytes = response.body().to_vec();
        let body_string = String::from_utf8(body_bytes).unwrap();

        assert_eq!(
            body_string,
            "Hello strava-oauth-exchange, this is an AWS Lambda HTTP request"
        );
    }
}
