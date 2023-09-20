use thiserror::Error;

const WARM_UP_LIMIT: i16 = 900;
const FIRST_SEGMENT_LIMIT: i16 = 2700;
const LAST_SEGMENT_LIMIT: i16 = 4500;

#[derive(PartialEq, Error, Debug)]
pub enum HeartRateDriftError {
    #[error("Not enough samples to calculate drift. For the calculation you need a 15 min warm-up, and two 30 min segments, so at least three samples")]
    NotEnoughSamples,
}

#[derive(PartialEq, Debug)]
pub struct HeartRateAtTime {
    heart_rate: i16,
    time: i16,
}

pub trait HeartRateDrift {
    fn heart_rate_drift(&self) -> Result<f32, HeartRateDriftError>;
}

impl HeartRateDrift for Vec<HeartRateAtTime> {
    fn heart_rate_drift(&self) -> Result<f32, HeartRateDriftError> {
        let first_segment: Vec<i16> = self
            .iter()
            .filter(|sample| sample.time >= WARM_UP_LIMIT && sample.time < FIRST_SEGMENT_LIMIT)
            .map(|sample| sample.heart_rate)
            .collect();

        let second_segment: Vec<i16> = self
            .iter()
            .filter(|sample| sample.time >= FIRST_SEGMENT_LIMIT && sample.time < LAST_SEGMENT_LIMIT)
            .map(|sample| sample.heart_rate)
            .collect();

        if first_segment.is_empty() || second_segment.is_empty() {
            Err(HeartRateDriftError::NotEnoughSamples)
        } else {
            let first_heart_rate_total: f32 = first_segment.iter().sum::<i16>().into();
            let second_heart_rate_total: f32 = second_segment.iter().sum::<i16>().into();
            let avg_heart_rate_first = first_heart_rate_total / first_segment.len() as f32;
            let avg_heart_rate_second = second_heart_rate_total / second_segment.len() as f32;

            let drift =
                ((avg_heart_rate_second - avg_heart_rate_first) / avg_heart_rate_first) * 100.0;
            Ok(drift)
        }
    }
}

pub fn combine_hr_with_time(heart_rates: &[i16], times: &[i16]) -> Vec<HeartRateAtTime> {
    heart_rates
        .iter()
        .copied()
        .zip(times.iter().copied())
        .map(|(rate, time)| HeartRateAtTime {
            heart_rate: rate,
            time,
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
}
