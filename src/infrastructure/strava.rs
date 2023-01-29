mod http;

use ::http::response::Builder;
use reqwest::{Error, Response};
use serde::Deserialize;
use thiserror::Error;

#[derive(Deserialize)]
pub struct Samples {
    data: Vec<i16>,
}

#[derive(Deserialize)]
pub struct StravaData {
    heartrate: Samples,
    time: Samples,
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

pub trait StravaClient {
    async fn request(&self) -> Result<Response, Error>;
}

pub struct Strava<T: StravaClient> {
    strava_client: T,
}

impl<T: StravaClient> Strava<T> {
    fn create_null(params: NullClient) -> Strava<NullClient> {
        Strava {
            strava_client: params,
        }
    }

    // At this point this works with NullClient, but only because
    // it doesn't use the response and instead returns empty vectors of JSON
    // You need to:
    //   - pass in the activity and the token
    // I'll use the integration test to make sure those work, but using a simple server
    //   - See the README for the correct URL and token
    //   - Just a note but I don't think you need an integration tests anymore
    // Eventually handle errors
    pub async fn get_activity_heart_rate(
        &self,
    ) -> Result<HeartRateSamples, ErrorGettingHeartRateData> {
        let res = self
            .strava_client
            .request()
            .await
            .expect("BE GOOD")
            .json::<StravaData>()
            .await
            .expect("Be JSON");

        Ok(HeartRateSamples {
            rates: res.heartrate.data,
            times: res.time.data,
        })
    }
}

struct NullClient {
    valid_response: Option<&'static str>,
    error: Option<Error>,
}

impl NullClient {
    fn response_from_valid_response_str(&self) -> Response {
        let response = Builder::new()
            .status(200)
            .body(self.valid_response.unwrap())
            .unwrap();
        Response::from(response)
    }
}

impl StravaClient for NullClient {
    async fn request(&self) -> Result<Response, Error> {
        Ok(self.response_from_valid_response_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_null_strava_client_with_response(response: &'static str) -> Strava<NullClient> {
        Strava::<NullClient>::create_null(NullClient {
            valid_response: Some(response),
            error: None,
        })
    }

    #[tokio::test]
    async fn get_activity_heart_rate_converts_empty_arrays() {
        let empty_json = "{
    \"heartrate\": {
        \"data\": []
    },
    \"time\": {
        \"data\": []
    }
}";
        let strava = create_null_strava_client_with_response(empty_json);

        let result = strava.get_activity_heart_rate().await.unwrap();

        assert!(result.rates.is_empty());
        assert!(result.times.is_empty());
    }

    #[tokio::test]
    async fn get_activity_heart_rate_converts_arrays_with_one_value() {
        let empty_json = "{
    \"heartrate\": {
        \"data\": [2]
    },
    \"time\": {
        \"data\": [3]
    }
}";
        let strava = create_null_strava_client_with_response(empty_json);

        let result = strava.get_activity_heart_rate().await.unwrap();

        assert_eq!(result.rates, vec![2]);
        assert_eq!(result.times, vec![3]);
    }
}
