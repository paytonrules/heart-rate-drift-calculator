use reqwest::{Error, Response};
use std::future::Future;
use thiserror::Error;

pub trait StravaClient {
    fn request() -> Box<dyn Future<Output = Result<Response, Error>>>;
}

pub struct Strava<T: StravaClient> {
    strava_client: T,
}

pub struct HeartRateSamples {
    pub rates: Vec<i16>,
    pub times: Vec<i16>,
}

#[derive(Debug, Error)]
pub enum ErrorGettingHeartRateData {
    #[error("Error connecting to Strava")]
    ConnectionError,
}

impl<T: StravaClient> Strava<T> {
    fn create_null(params: NullClient) -> Strava<NullClient> {
        Strava {
            strava_client: params,
        }
    }

    // Untested, unworking, etc
    pub async fn get_activity_heart_rate(
        &self,
    ) -> Result<HeartRateSamples, ErrorGettingHeartRateData> {
        let token = "temp";
        let client = reqwest::ClientBuilder::new().build().expect("BOOM?");
        let res = client
            .get("https://www.strava.com/api/v3/activities/7944016770/streams?keys=heartrate,time&key_by_type=true")
            .header("Authorization", "Bearer ".to_owned() + &token)
            .send()
            .await;

        Ok(HeartRateSamples {
            rates: vec![],
            times: vec![],
        })
    }
}

struct NullClient {
    response: Option<Result<Response, Error>>,
}

impl StravaClient for NullClient {
    fn request() -> Box<dyn Future<Output = Result<Response, Error>>> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use http::response::Builder;
    use reqwest::Response;

    fn create_reqwest_valid_response(body: &str) -> Response {
        let response = Builder::new().status(200).body("foo").unwrap();
        Response::from(response)
    }

    #[tokio::test]
    async fn null_strava_client_returns_passed_in_value() {
        let empty_json = "{
    \"heartrate\": {
        \"data\": []
    },
    \"time\": {
        \"data\": []
    }
};";

        let response = create_reqwest_valid_response(empty_json);

        let strava = Strava::<NullClient>::create_null(NullClient {
            response: Some(Ok(response)),
        });

        let result = strava.get_activity_heart_rate().await.unwrap();

        assert!(result.rates.is_empty());
        assert!(result.times.is_empty());
    }
}
