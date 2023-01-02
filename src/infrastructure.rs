use reqwest::{Error, Response};
use thiserror::Error;

pub struct Strava;

pub struct HeartRateSamples {
    pub rates: Vec<i16>,
    pub times: Vec<i16>,
}

#[derive(Debug, Error)]
pub enum ErrorGettingHeartRateData {
    #[error("Error connecting to Strava")]
    ConnectionError,
}

struct NullParams {
    response: Option<Result<Response, Error>>,
}

impl Strava {
    fn create_null(params: NullParams) -> Self {
        Strava
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

        let strava = Strava::create_null(NullParams {
            response: Some(Ok(response)),
        });

        let result = strava.get_activity_heart_rate().await.unwrap();

        assert!(result.rates.is_empty());
        assert!(result.times.is_empty());
    }
}
