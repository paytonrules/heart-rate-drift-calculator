use thiserror::Error;

fn main() {
    // Sign into strava
    // Make API call to it to get the info from the given race
    // Calculate HR drift
    println!("Hello, world!");
}

const WARM_UP_LIMIT: i16 = 900;
const FIRST_SEGMENT_LIMIT: i16 = 2700;
const LAST_SEGMENT_LIMIT: i16 = 4500;

#[derive(PartialEq, Error, Debug)]
enum HeartRateDriftError {
    #[error("Not enough samples to calculate drift. For the calculation you need a 15 min warm-up, and two 30 min segments, so at least three samples")]
    NotEnoughSamples,
}

#[derive(PartialEq, Debug)]
struct HeartRateAtTime {
    heart_rate: i16,
    time: i16,
}

trait HeartRateDrift {
    fn heart_rate_drift(&self) -> Result<f32, HeartRateDriftError>;
}

impl HeartRateDrift for Vec<HeartRateAtTime> {
    fn heart_rate_drift(&self) -> Result<f32, HeartRateDriftError> {
        let without_warm_up = self.iter().filter(|sample| sample.time >= WARM_UP_LIMIT);

        let first_segment = without_warm_up
            .clone()
            .filter(|sample| sample.time < FIRST_SEGMENT_LIMIT);

        let second_segment = without_warm_up.filter(|sample| {
            sample.time >= FIRST_SEGMENT_LIMIT && sample.time < LAST_SEGMENT_LIMIT
        });

        if first_segment.clone().peekable().peek() == None
            || second_segment.clone().peekable().peek() == None
        {
            Err(HeartRateDriftError::NotEnoughSamples)
        } else {
            let first_heart_rate: i16 = first_segment.clone().map(|sample| sample.heart_rate).sum();
            let second_heart_rate: i16 =
                second_segment.clone().map(|sample| sample.heart_rate).sum();
            let avg_heart_rate_first: f32 = first_heart_rate as f32 / first_segment.count() as f32;
            let avg_heart_rate_second: f32 =
                second_heart_rate as f32 / second_segment.count() as f32;

            let drift: f32 =
                ((avg_heart_rate_second - avg_heart_rate_first) / avg_heart_rate_first) * 100.0;
            Ok(drift)
        }
    }
}

fn combine_hr_with_time(heart_rates: &Vec<i16>, times: &Vec<i16>) -> Vec<HeartRateAtTime> {
    heart_rates
        .iter()
        .copied()
        .zip(times.iter().copied())
        .map(|(rate, time)| HeartRateAtTime {
            heart_rate: rate,
            time: time,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_couple_heart_rates_and_times() {
        let heart_rates = vec![0, 1, 2];
        let times = vec![3, 4, 5];
        let actual_vec = combine_hr_with_time(&heart_rates, &times);

        let expected_vec = vec![
            HeartRateAtTime {
                heart_rate: 0,
                time: 3,
            },
            HeartRateAtTime {
                heart_rate: 1,
                time: 4,
            },
            HeartRateAtTime {
                heart_rate: 2,
                time: 5,
            },
        ];
        assert_eq!(expected_vec, actual_vec);
    }

    #[test]
    fn test_error_not_enough_samples_for_heart_rate_drift() {
        let samples = vec![];

        assert_eq!(
            Err(HeartRateDriftError::NotEnoughSamples),
            samples.heart_rate_drift()
        );
    }

    #[test]
    fn test_one_sample_is_not_enough_samples_for_heart_rate_drift() {
        let samples = vec![HeartRateAtTime {
            heart_rate: 0,
            time: 1,
        }];

        assert_eq!(
            Err(HeartRateDriftError::NotEnoughSamples),
            samples.heart_rate_drift()
        );
    }

    #[test]
    fn test_three_samples_one_in_each_window_is_enough() {
        let samples = vec![
            HeartRateAtTime {
                heart_rate: 1,
                time: WARM_UP_LIMIT - 1,
            },
            HeartRateAtTime {
                heart_rate: 1,
                time: FIRST_SEGMENT_LIMIT - 1,
            },
            HeartRateAtTime {
                heart_rate: 1,
                time: LAST_SEGMENT_LIMIT - 1,
            },
        ];

        assert_eq!(Ok(0.0), samples.heart_rate_drift());
    }

    #[test]
    fn test_missing_sample_in_last_segment_is_an_error() {
        let samples = vec![
            HeartRateAtTime {
                heart_rate: 1,
                time: WARM_UP_LIMIT - 1,
            },
            HeartRateAtTime {
                heart_rate: 1,
                time: FIRST_SEGMENT_LIMIT - 2,
            },
            HeartRateAtTime {
                heart_rate: 1,
                time: FIRST_SEGMENT_LIMIT - 1,
            },
        ];

        assert_eq!(
            Err(HeartRateDriftError::NotEnoughSamples),
            samples.heart_rate_drift()
        );
    }

    #[test]
    fn test_missing_sample_in_first_segment_is_an_error() {
        let samples = vec![
            HeartRateAtTime {
                heart_rate: 1,
                time: WARM_UP_LIMIT - 1,
            },
            HeartRateAtTime {
                heart_rate: 1,
                time: FIRST_SEGMENT_LIMIT + 1,
            },
            HeartRateAtTime {
                heart_rate: 1,
                time: FIRST_SEGMENT_LIMIT + 2,
            },
        ];

        assert_eq!(
            Err(HeartRateDriftError::NotEnoughSamples),
            samples.heart_rate_drift()
        );
    }

    #[test]
    fn test_missing_sample_in_warm_up_is_fine() {
        let samples = vec![
            HeartRateAtTime {
                heart_rate: 1,
                time: WARM_UP_LIMIT + 1,
            },
            HeartRateAtTime {
                heart_rate: 1,
                time: FIRST_SEGMENT_LIMIT + 1,
            },
        ];

        assert_eq!(Ok(0.0), samples.heart_rate_drift());
    }

    #[test]
    fn test_calculate_heart_rate_drift_of_valid_samples_via_percentage_rise_between_averages() {
        let samples = vec![
            HeartRateAtTime {
                heart_rate: 1,
                time: WARM_UP_LIMIT + 1,
            },
            HeartRateAtTime {
                heart_rate: 2,
                time: FIRST_SEGMENT_LIMIT + 1,
            },
        ];

        assert_eq!(Ok(100.0), samples.heart_rate_drift());
    }

    // Test an actual calculation
    // Test that WARM_UP_LIMIT is in the first segment and FIRST_SEGMENT_LIMIT is not
    // Test that FIRST_SEGMENT_LIMIT is in the second segment and LAST_SEGMENT_LIMIT is not
    // Test that anything after LAST_SEGMENT_LIMIT is ignored
}

/* Example data:

Where you need the activity ID, and you need to make sure your access token (which is in the Auth header)
Has activity:read_all scope. You might need to switch it from your normal token via the directions here:

https://jessicasalbert.medium.com/holding-your-hand-through-stravas-api-e642d15695f2

That part is unclear.

Query is - https://www.strava.com/api/v3/activities/7944016770/streams?keys=heartrate,time&key_by_type=true

You'll get back two streams - HR and time that look like this:

"heartrate": {
"data": [
80,
83,
89,
92,
...]

and
"time": {
"data": [
0,
2,
4,
5,
7,
10,
...]

Those should have the same resolution - I hope. They did in my first test query anyway. So just sync those up, trim the first 15
min, and you got it.
*/
