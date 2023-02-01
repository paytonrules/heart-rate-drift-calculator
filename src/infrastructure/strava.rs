mod http;

use self::http::{AuthToken, Client, NullClient, SimpleHttpClient, Url};
use serde::Deserialize;
use thiserror::Error;

const STRAVA_API: &str = "https://www.strava.com/api/v3/activities";

pub struct ActivityID(String);

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

pub struct Strava<T: SimpleHttpClient> {
    strava_client: Client<T>,
}

impl<T: SimpleHttpClient> Strava<T> {
    pub async fn get_activity_heart_rate(
        &self,
        token: &AuthToken,
        activity: &ActivityID,
    ) -> Result<HeartRateSamples, ErrorGettingHeartRateData> {
        let full_url = construct_activity_url(activity);

        let res = self
            .strava_client
            .request(&Url(full_url), token)
            .await
            .expect("BE GOOD") // TODO: Fix!
            .json::<StravaData>()
            .await
            .expect("Be JSON"); // TODO: Fix!

        Ok(HeartRateSamples {
            rates: res.heartrate.data,
            times: res.time.data,
        })
    }
}

impl Strava<NullClient> {
    fn null() -> Self {
        Self {
            strava_client: Client::create_null(),
        }
    }

    fn with_activity(
        self,
        token: AuthToken,
        activity: &ActivityID,
        response: &'static str,
    ) -> Self {
        let full_url = construct_activity_url(activity);
        Self {
            strava_client: self.strava_client.map_authenticated_url(
                token,
                Url(full_url),
                response.to_string(),
            ),
        }
    }
}

fn construct_activity_url(activity: &ActivityID) -> String {
    format!("https://{STRAVA_API}/{}", activity.0)
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let token = AuthToken("token".to_string());
        let activity = ActivityID("activity-id".to_string());
        let strava = Strava::null().with_activity(token.clone(), &activity, empty_json);

        let result = strava
            .get_activity_heart_rate(&token, &activity)
            .await
            .unwrap();

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

        let token = AuthToken("token".to_string());
        let activity = ActivityID("activity-id".to_string());
        let strava = Strava::null().with_activity(token.clone(), &activity, empty_json);

        let result = strava
            .get_activity_heart_rate(&token, &activity)
            .await
            .unwrap();

        assert_eq!(result.rates, vec![2]);
        assert_eq!(result.times, vec![3]);
    }
}
